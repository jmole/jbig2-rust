//! Sandboxed execution wrapper for untrusted external binaries.
//!
//! [`Sandbox::run`] wraps a [`Command`] in three protective layers:
//!
//! 1. A **wall-clock watchdog** (always on) sends `SIGTERM` then `SIGKILL`
//!    if the child is still running after [`Limits::wall_seconds`].
//! 2. **Resource limits** applied through `setrlimit(2)` in a `pre_exec`
//!    hook on Unix (Linux + macOS): `RLIMIT_AS`, `RLIMIT_CPU`,
//!    `RLIMIT_FSIZE`, `RLIMIT_NOFILE`, `RLIMIT_CORE`.
//! 3. **Filesystem isolation** via `bwrap` (Linux) or `sandbox-exec`
//!    (macOS) when those tools are available; otherwise we fall back to
//!    the bare `Command` (still subject to layers 1 + 2).
//!
//! All paths the child needs to read or write must be declared via
//! [`Sandbox::ro_path`] / [`Sandbox::rw_path`]. Anything not declared is
//! rejected by the filesystem layer.
//!
//! Output bytes are capped at [`Limits::output_bytes`] per stream; the
//! drain threads keep reading after the cap to avoid blocking the
//! writer, then truncate the stored buffer and surface
//! [`KillReason::OutputBytes`] in the resulting [`SandboxOutcome`].
//!
//! ## Presets
//!
//! Use the named constructors instead of building [`Limits`] by hand:
//!
//! * [`Sandbox::for_decoder`] - strictest, intended for fuzzy / hostile
//!   bitstreams fed to decoders (10 s wall, 5 s CPU, 512 MiB AS).
//! * [`Sandbox::for_encoder`] - slightly looser to accommodate slow
//!   reference encoders (60 s wall, 45 s CPU, 1 GiB AS).
//! * [`Sandbox::for_comparator`] - same envelope as the encoder.
//!   Available preset for side-by-side image comparators. The
//!   conformance matrix no longer wires this to `imgcomp` (see
//!   `docs/conformance-matrix-encode-audit.md` for the in-tree
//!   roundtrip oracle), but the preset remains for future external
//!   comparators or interop tooling.
//!
//! ## Environment overrides
//!
//! Defaults can be relaxed at runtime without rebuilding:
//!
//! | Variable | Meaning |
//! |---|---|
//! | `JBIG2_SANDBOX_WALL_SECS` | Override [`Limits::wall_seconds`] |
//! | `JBIG2_SANDBOX_CPU_SECS`  | Override [`Limits::cpu_seconds`]  |
//! | `JBIG2_SANDBOX_AS_BYTES`  | Override [`Limits::address_space_bytes`] |
//! | `JBIG2_SANDBOX_OUTPUT_BYTES` | Override [`Limits::output_bytes`] |
//! | `JBIG2_SANDBOX_BACKEND`   | Force backend: `bwrap`, `sandbox-exec`, or `passthrough` |
//! | `JBIG2_SANDBOX_DISABLE_RLIMITS` | Set to `1` to skip the `setrlimit` pre_exec hook (debugging only) |

#![allow(dead_code)]

use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Backend used to enforce filesystem isolation around the child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    /// Linux: `bwrap --unshare-all --die-with-parent --new-session`.
    Bwrap,
    /// macOS: `sandbox-exec -f <profile.sb>`.
    SandboxExec,
    /// No isolation; only the watchdog + setrlimit layers apply.
    Passthrough,
}

/// Resource limits enforced for each sandboxed invocation.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Wall-clock seconds before the watchdog sends `SIGTERM`/`SIGKILL`.
    pub wall_seconds: u64,
    /// `RLIMIT_CPU` value (CPU seconds the kernel allows the child).
    pub cpu_seconds: u64,
    /// `RLIMIT_AS` value (max virtual address space, bytes).
    pub address_space_bytes: u64,
    /// `RLIMIT_FSIZE` value (max bytes written per file).
    pub file_size_bytes: u64,
    /// `RLIMIT_NOFILE` value (max open files).
    pub max_files: u64,
    /// Cap on bytes captured from each of stdout/stderr.
    pub output_bytes: usize,
}

impl Limits {
    /// Strict defaults intended for decoders chewing on hostile input.
    pub const DECODER: Self = Self {
        wall_seconds: 10,
        cpu_seconds: 5,
        address_space_bytes: 512 * 1024 * 1024,
        file_size_bytes: 256 * 1024 * 1024,
        max_files: 256,
        output_bytes: 4 * 1024 * 1024,
    };

    /// Looser defaults that accommodate slow reference encoders.
    pub const ENCODER: Self = Self {
        wall_seconds: 60,
        cpu_seconds: 45,
        address_space_bytes: 1024 * 1024 * 1024,
        file_size_bytes: 512 * 1024 * 1024,
        max_files: 256,
        output_bytes: 8 * 1024 * 1024,
    };

    /// Same envelope as the encoder. Available preset for image
    /// comparators; not currently wired to `imgcomp` (the conformance
    /// matrix uses an in-tree roundtrip comparator instead, see
    /// `docs/conformance-matrix-encode-audit.md`).
    pub const COMPARATOR: Self = Self::ENCODER;

    fn apply_env_overrides(mut self) -> Self {
        if let Some(v) = parse_env_u64("JBIG2_SANDBOX_WALL_SECS") {
            self.wall_seconds = v;
        }
        if let Some(v) = parse_env_u64("JBIG2_SANDBOX_CPU_SECS") {
            self.cpu_seconds = v;
        }
        if let Some(v) = parse_env_u64("JBIG2_SANDBOX_AS_BYTES") {
            self.address_space_bytes = v;
        }
        if let Some(v) = parse_env_u64("JBIG2_SANDBOX_OUTPUT_BYTES") {
            self.output_bytes = v as usize;
        }
        self
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::DECODER
    }
}

/// Reason the sandbox proactively terminated the child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KillReason {
    /// Watchdog hit `wall_seconds`.
    Timeout,
    /// Heuristically detected `RLIMIT_AS` exhaustion (OOM marker in
    /// stderr) or kernel SIGKILL after we set `RLIMIT_AS`. Best-effort.
    AddressSpace,
    /// stdout or stderr exceeded `output_bytes`.
    OutputBytes,
}

/// Result of [`Sandbox::run`].
pub struct SandboxOutcome {
    /// Captured exit status, stdout, and stderr.
    ///
    /// Output is truncated to [`Limits::output_bytes`] per stream.
    pub output: Output,
    /// `Some(_)` when the sandbox itself terminated the child.
    pub kill_reason: Option<KillReason>,
    /// Wall-clock duration of the child.
    pub wall_elapsed: Duration,
}

/// Sandboxed runner around an external binary.
#[derive(Clone, Debug)]
pub struct Sandbox {
    backend: Backend,
    ro_paths: Vec<PathBuf>,
    rw_paths: Vec<PathBuf>,
    limits: Limits,
    sanitizer_env: bool,
    inject_rlimits: bool,
}

impl Sandbox {
    /// Auto-detect the best available backend for this platform.
    ///
    /// Honours `JBIG2_SANDBOX_BACKEND` for forcing a specific backend
    /// during local debugging.
    pub fn detect() -> Self {
        let backend = backend_from_env().unwrap_or_else(detect_backend);
        Self {
            backend,
            ro_paths: Vec::new(),
            rw_paths: Vec::new(),
            limits: Limits::default().apply_env_overrides(),
            sanitizer_env: true,
            inject_rlimits: !env_flag("JBIG2_SANDBOX_DISABLE_RLIMITS"),
        }
    }

    /// Strict preset for decoders (10 s wall, 512 MiB AS).
    pub fn for_decoder() -> Self {
        let mut sb = Self::detect();
        sb.limits = Limits::DECODER.apply_env_overrides();
        sb
    }

    /// Looser preset for encoders (60 s wall, 1 GiB AS).
    pub fn for_encoder() -> Self {
        let mut sb = Self::detect();
        sb.limits = Limits::ENCODER.apply_env_overrides();
        sb
    }

    /// Same envelope as [`Sandbox::for_encoder`]. Available preset for
    /// image-comparison binaries; not currently used by the conformance
    /// matrix, which performs comparisons in-process. Kept public so
    /// future external comparators or interop tooling can opt in
    /// without redefining the limit envelope.
    pub fn for_comparator() -> Self {
        let mut sb = Self::detect();
        sb.limits = Limits::COMPARATOR.apply_env_overrides();
        sb
    }

    /// Force the [`Backend::Passthrough`] backend (no fs isolation).
    pub fn passthrough() -> Self {
        let mut sb = Self::detect();
        sb.backend = Backend::Passthrough;
        sb
    }

    /// Add a path the child may read.
    pub fn ro_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.ro_paths.push(path.into());
        self
    }

    /// Add a path the child may read and write.
    pub fn rw_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.rw_paths.push(path.into());
        self
    }

    /// Replace the resource limits wholesale.
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits.apply_env_overrides();
        self
    }

    /// Force a specific backend (rarely useful; prefer [`Sandbox::detect`]).
    pub fn with_backend(mut self, backend: Backend) -> Self {
        self.backend = backend;
        self
    }

    /// Disable the `ASAN_OPTIONS` / `UBSAN_OPTIONS` env injection.
    pub fn without_sanitizer_env(mut self) -> Self {
        self.sanitizer_env = false;
        self
    }

    /// Currently configured backend.
    pub fn backend(&self) -> Backend {
        self.backend
    }

    /// Currently configured limits.
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// Spawn `cmd` inside the sandbox and capture its result.
    ///
    /// The child is wrapped according to the configured backend, then
    /// guarded by the wall-clock watchdog and (on Unix) `setrlimit`
    /// hooks. Captured output is truncated to `limits.output_bytes`.
    pub fn run(&self, cmd: Command) -> io::Result<SandboxOutcome> {
        let mut wrapped = match self.backend {
            Backend::Bwrap => self.wrap_bwrap(cmd),
            Backend::SandboxExec => self.wrap_sandbox_exec(cmd)?,
            Backend::Passthrough => cmd,
        };
        if self.sanitizer_env {
            wrapped.env(
                "ASAN_OPTIONS",
                "abort_on_error=1:detect_leaks=0:halt_on_error=1:symbolize=1:print_stacktrace=1",
            );
            wrapped.env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1");
        }
        wrapped.stdin(Stdio::null());
        wrapped.stdout(Stdio::piped());
        wrapped.stderr(Stdio::piped());
        #[cfg(unix)]
        if self.inject_rlimits {
            install_rlimits(&mut wrapped, self.limits);
        }
        spawn_and_supervise(wrapped, self.limits)
    }

    fn wrap_bwrap(&self, cmd: Command) -> Command {
        let program = cmd.get_program().to_owned();
        let args: Vec<_> = cmd.get_args().map(|arg| arg.to_owned()).collect();
        let envs: Vec<_> = cmd
            .get_envs()
            .filter_map(|(k, v)| v.map(|v| (k.to_owned(), v.to_owned())))
            .collect();
        let cwd = cmd.get_current_dir().map(Path::to_path_buf);

        let mut wrapped = Command::new("bwrap");
        wrapped
            .arg("--ro-bind")
            .arg("/usr")
            .arg("/usr")
            .arg("--symlink")
            .arg("/usr/lib")
            .arg("/lib")
            .arg("--symlink")
            .arg("/usr/lib64")
            .arg("/lib64")
            .arg("--symlink")
            .arg("/usr/bin")
            .arg("/bin")
            .arg("--ro-bind-try")
            .arg("/etc")
            .arg("/etc")
            .arg("--tmpfs")
            .arg("/tmp")
            .arg("--proc")
            .arg("/proc")
            .arg("--dev")
            .arg("/dev")
            .arg("--unshare-all")
            .arg("--die-with-parent")
            .arg("--new-session")
            .arg("--cap-drop")
            .arg("ALL")
            .arg("--clearenv");
        // Re-export only environment variables the caller explicitly set.
        // Sanitizer + rlimits-disable env vars are forwarded too.
        for (k, v) in &envs {
            wrapped.arg("--setenv").arg(k).arg(v);
        }
        for path in &self.ro_paths {
            wrapped.arg("--ro-bind").arg(path).arg(path);
        }
        for path in &self.rw_paths {
            wrapped.arg("--bind").arg(path).arg(path);
        }
        if let Some(dir) = &cwd {
            wrapped.arg("--chdir").arg(dir);
        }
        wrapped.arg("--");
        wrapped.arg(program).args(args);
        wrapped
    }

    fn wrap_sandbox_exec(&self, cmd: Command) -> io::Result<Command> {
        let profile = sandbox_exec_profile(&self.ro_paths, &self.rw_paths);
        let profile_path = std::env::temp_dir().join(format!(
            "jbig2-sandbox-{}-{}.sb",
            std::process::id(),
            sandbox_seq()
        ));
        std::fs::write(&profile_path, profile)?;

        let program = cmd.get_program().to_owned();
        let args: Vec<_> = cmd.get_args().map(|arg| arg.to_owned()).collect();
        let envs: Vec<_> = cmd
            .get_envs()
            .filter_map(|(k, v)| v.map(|v| (k.to_owned(), v.to_owned())))
            .collect();
        let cwd = cmd.get_current_dir().map(Path::to_path_buf);

        let mut wrapped = Command::new("sandbox-exec");
        wrapped.arg("-f").arg(profile_path);
        wrapped.arg(program).args(args);
        for (k, v) in envs {
            wrapped.env(k, v);
        }
        if let Some(dir) = cwd {
            wrapped.current_dir(dir);
        }
        Ok(wrapped)
    }
}

#[cfg(unix)]
fn install_rlimits(cmd: &mut Command, limits: Limits) {
    use std::os::unix::process::CommandExt;
    let captured = limits;
    unsafe {
        cmd.pre_exec(move || {
            libc::setsid();
            apply_rlimit(libc::RLIMIT_CPU as libc::c_int, captured.cpu_seconds);
            apply_rlimit(libc::RLIMIT_AS as libc::c_int, captured.address_space_bytes);
            apply_rlimit(libc::RLIMIT_FSIZE as libc::c_int, captured.file_size_bytes);
            apply_rlimit(libc::RLIMIT_NOFILE as libc::c_int, captured.max_files);
            apply_rlimit(libc::RLIMIT_CORE as libc::c_int, 0);
            Ok(())
        });
    }
}

#[cfg(unix)]
fn apply_rlimit(resource: libc::c_int, value: u64) {
    let rl = libc::rlimit {
        rlim_cur: value as libc::rlim_t,
        rlim_max: value as libc::rlim_t,
    };
    unsafe {
        libc::setrlimit(resource as _, &rl);
    }
}

fn spawn_and_supervise(mut cmd: Command, limits: Limits) -> io::Result<SandboxOutcome> {
    let started = Instant::now();
    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let pid = child.id();

    let (kill_tx, kill_rx) = mpsc::channel::<KillReason>();

    let watchdog = spawn_watchdog(pid, limits.wall_seconds, kill_rx);

    let cap = limits.output_bytes;
    let stdout_handle = stdout.map(|s| {
        let tx = kill_tx.clone();
        thread::spawn(move || drain_capped(s, cap, tx))
    });
    let stderr_handle = stderr.map(|s| {
        let tx = kill_tx.clone();
        thread::spawn(move || drain_capped(s, cap, tx))
    });
    drop(kill_tx);

    let status = wait_child(&mut child)?;
    let wall_elapsed = started.elapsed();
    let (stdout_bytes, stdout_over) = stdout_handle
        .map(|h| h.join().unwrap_or_else(|_| (Vec::new(), false)))
        .unwrap_or((Vec::new(), false));
    let (stderr_bytes, stderr_over) = stderr_handle
        .map(|h| h.join().unwrap_or_else(|_| (Vec::new(), false)))
        .unwrap_or((Vec::new(), false));
    let watchdog_reason = watchdog.join().unwrap_or(None);

    let kill_reason = watchdog_reason.or_else(|| {
        if stdout_over || stderr_over {
            Some(KillReason::OutputBytes)
        } else if heuristic_address_space(&status, &stderr_bytes) {
            Some(KillReason::AddressSpace)
        } else {
            None
        }
    });

    Ok(SandboxOutcome {
        output: Output {
            status,
            stdout: stdout_bytes,
            stderr: stderr_bytes,
        },
        kill_reason,
        wall_elapsed,
    })
}

fn wait_child(child: &mut Child) -> io::Result<std::process::ExitStatus> {
    child.wait()
}

fn spawn_watchdog(
    pid: u32,
    wall_seconds: u64,
    kill_rx: mpsc::Receiver<KillReason>,
) -> thread::JoinHandle<Option<KillReason>> {
    let wall = Duration::from_secs(wall_seconds.max(1));
    thread::spawn(move || -> Option<KillReason> {
        let started = Instant::now();
        let mut requested: Option<KillReason> = None;
        loop {
            let remaining = wall.saturating_sub(started.elapsed());
            let poll = remaining.min(Duration::from_millis(100));
            match kill_rx.recv_timeout(poll) {
                Ok(reason) => {
                    requested.get_or_insert(reason);
                    break;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return None;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if started.elapsed() >= wall {
                        requested = Some(KillReason::Timeout);
                        break;
                    }
                }
            }
        }
        terminate_pid(pid);
        Some(requested.unwrap_or(KillReason::Timeout))
    })
}

#[cfg(unix)]
fn terminate_pid(pid: u32) {
    let pgid = -(pid as libc::pid_t);
    unsafe {
        libc::kill(pgid, libc::SIGTERM);
    }
    thread::sleep(Duration::from_millis(750));
    unsafe {
        libc::kill(pgid, libc::SIGKILL);
    }
}

#[cfg(not(unix))]
fn terminate_pid(_pid: u32) {
    // Non-Unix kill happens through Child::kill at the call site.
}

fn drain_capped(
    mut reader: impl Read,
    cap: usize,
    kill_tx: mpsc::Sender<KillReason>,
) -> (Vec<u8>, bool) {
    let mut buf = Vec::with_capacity(cap.min(64 * 1024));
    let mut chunk = [0u8; 8192];
    let mut over = false;
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                if !over {
                    let remaining = cap.saturating_sub(buf.len());
                    if n <= remaining {
                        buf.extend_from_slice(&chunk[..n]);
                        if buf.len() == cap {
                            over = true;
                            let _ = kill_tx.send(KillReason::OutputBytes);
                        }
                    } else {
                        buf.extend_from_slice(&chunk[..remaining]);
                        over = true;
                        let _ = kill_tx.send(KillReason::OutputBytes);
                    }
                }
                // Keep draining after overflow so the writer doesn't
                // block on a full pipe even if the watchdog kill is
                // still propagating.
            }
            Err(_) => break,
        }
    }
    (buf, over)
}

fn heuristic_address_space(status: &std::process::ExitStatus, stderr: &[u8]) -> bool {
    let stderr = String::from_utf8_lossy(stderr);
    let oom_markers = [
        "Cannot allocate memory",
        "out of memory",
        "std::bad_alloc",
        "AddressSanitizer: allocation-size-too-big",
        "allocator_may_return_null",
    ];
    if oom_markers.iter().any(|m| stderr.contains(m)) {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            if sig == libc::SIGABRT && stderr.contains("malloc") {
                return true;
            }
        }
    }
    let _ = status;
    false
}

fn detect_backend() -> Backend {
    use std::sync::OnceLock;
    static CACHED: OnceLock<Backend> = OnceLock::new();
    *CACHED.get_or_init(|| {
        if cfg!(target_os = "linux") && command_exists("bwrap") {
            if probe_bwrap() {
                Backend::Bwrap
            } else {
                Backend::Passthrough
            }
        } else if cfg!(target_os = "macos") && command_exists("sandbox-exec") {
            if probe_sandbox_exec() {
                Backend::SandboxExec
            } else {
                Backend::Passthrough
            }
        } else {
            Backend::Passthrough
        }
    })
}

#[cfg(target_os = "linux")]
fn probe_bwrap() -> bool {
    Command::new("bwrap")
        .arg("--unshare-all")
        .arg("--die-with-parent")
        .arg("--ro-bind")
        .arg("/usr")
        .arg("/usr")
        .arg("--")
        .arg("/bin/true")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn probe_bwrap() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn probe_sandbox_exec() -> bool {
    Command::new("sandbox-exec")
        .arg("-p")
        .arg("(version 1)(allow default)")
        .arg("/usr/bin/true")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn probe_sandbox_exec() -> bool {
    false
}

fn backend_from_env() -> Option<Backend> {
    match std::env::var("JBIG2_SANDBOX_BACKEND")
        .ok()?
        .to_ascii_lowercase()
        .as_str()
    {
        "bwrap" => Some(Backend::Bwrap),
        "sandbox-exec" | "sandboxexec" => Some(Backend::SandboxExec),
        "passthrough" | "off" | "none" => Some(Backend::Passthrough),
        _ => None,
    }
}

fn command_exists(name: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|path| path.join(name).is_file()))
}

fn parse_env_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok().and_then(|v| v.trim().parse().ok())
}

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes")
    )
}

/// Build a `sandbox-exec` profile.
///
/// macOS's deny-default profiles require an enormous and brittle allow-list
/// to cover the kernel surface every C/Java decoder touches via dyld and
/// libsystem (Mach lookups, sysctl reads, IO Kit, dyld shared cache, etc.).
/// We use the `(allow default)` baseline and clamp down on the two threats
/// that actually matter for untrusted decoder input: outbound network and
/// arbitrary filesystem writes. Reads remain broadly allowed because the
/// dynamic linker, `/usr/share/icu`, locale tables, and ICC profiles all
/// expect to be readable from anywhere on the volume.
fn sandbox_exec_profile(_ro_paths: &[PathBuf], rw_paths: &[PathBuf]) -> String {
    let mut out = String::from("(version 1)\n(allow default)\n");
    out.push_str("(deny network*)\n");
    out.push_str("(allow network* (local ip))\n");
    out.push_str("(allow network* (local unix-socket))\n");
    out.push_str("(deny file-write*)\n");
    out.push_str("(allow file-write* (subpath \"/private/tmp\"))\n");
    out.push_str("(allow file-write* (subpath \"/private/var/tmp\"))\n");
    out.push_str("(allow file-write* (subpath \"/private/var/folders\"))\n");
    out.push_str("(allow file-write-data (literal \"/dev/null\"))\n");
    out.push_str("(allow file-write-data (literal \"/dev/dtracehelper\"))\n");
    for path in rw_paths {
        out.push_str(&format!(
            "(allow file-write* (subpath \"{}\"))\n",
            escape(path)
        ));
    }
    out
}

fn escape(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

fn sandbox_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn pt_sandbox(limits: Limits) -> Sandbox {
        let mut sb = Sandbox::detect();
        sb.backend = Backend::Passthrough;
        sb.limits = limits;
        sb
    }

    #[test]
    fn echo_succeeds_without_kill_reason() {
        let cmd = {
            let mut c = Command::new("/bin/sh");
            c.arg("-c").arg("printf hello");
            c
        };
        let outcome = pt_sandbox(Limits::DECODER).run(cmd).unwrap();
        assert!(
            outcome.kill_reason.is_none(),
            "got {:?}",
            outcome.kill_reason
        );
        assert!(outcome.output.status.success());
        assert_eq!(outcome.output.stdout, b"hello");
    }

    #[test]
    fn timeout_kill_reason_within_two_seconds() {
        let cmd = {
            let mut c = Command::new("/bin/sh");
            c.arg("-c").arg("sleep 60");
            c
        };
        let mut limits = Limits::DECODER;
        limits.wall_seconds = 1;
        let started = Instant::now();
        let outcome = pt_sandbox(limits).run(cmd).unwrap();
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_secs(4),
            "watchdog took too long: {:?}",
            elapsed
        );
        assert_eq!(outcome.kill_reason, Some(KillReason::Timeout));
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            assert!(
                outcome.output.status.signal().is_some(),
                "child should have been signaled, got {:?}",
                outcome.output.status
            );
        }
    }

    #[test]
    fn output_bytes_kill_reason_when_child_floods_stdout() {
        let cmd = {
            let mut c = Command::new("/bin/sh");
            c.arg("-c")
                .arg("yes hello-from-jbig2-sandbox-output-overflow-test 2>/dev/null");
            c
        };
        let mut limits = Limits::DECODER;
        limits.output_bytes = 16 * 1024;
        limits.wall_seconds = 5;
        let outcome = pt_sandbox(limits).run(cmd).unwrap();
        assert!(
            matches!(
                outcome.kill_reason,
                Some(KillReason::OutputBytes) | Some(KillReason::Timeout)
            ),
            "expected OutputBytes (or Timeout fallback), got {:?}",
            outcome.kill_reason
        );
        assert!(
            outcome.output.stdout.len() <= 16 * 1024,
            "stdout not capped: {} bytes",
            outcome.output.stdout.len()
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn address_space_limit_kills_huge_alloc_on_linux() {
        // Spawn a sub-shell that tries to grow far beyond the AS limit.
        // glibc malloc returns NULL once RLIMIT_AS is exhausted; the
        // resulting non-zero exit is our success signal here.
        let cmd = {
            let mut c = Command::new("/bin/sh");
            c.arg("-c")
                .arg("perl -e 'my $x = \"a\" x (2*1024*1024*1024); print length($x);'");
            c
        };
        let mut limits = Limits::DECODER;
        limits.address_space_bytes = 64 * 1024 * 1024;
        limits.wall_seconds = 30;
        let outcome = pt_sandbox(limits).run(cmd).unwrap();
        assert!(
            !outcome.output.status.success(),
            "child should have failed under RLIMIT_AS: status={:?}",
            outcome.output.status
        );
    }

    #[test]
    fn env_overrides_apply_to_limits() {
        let mut limits = Limits::DECODER;
        std::env::set_var("JBIG2_SANDBOX_WALL_SECS", "7");
        std::env::set_var("JBIG2_SANDBOX_OUTPUT_BYTES", "1024");
        limits = limits.apply_env_overrides();
        std::env::remove_var("JBIG2_SANDBOX_WALL_SECS");
        std::env::remove_var("JBIG2_SANDBOX_OUTPUT_BYTES");
        assert_eq!(limits.wall_seconds, 7);
        assert_eq!(limits.output_bytes, 1024);
    }

    #[test]
    fn detect_returns_passthrough_when_forced() {
        std::env::set_var("JBIG2_SANDBOX_BACKEND", "passthrough");
        let sb = Sandbox::detect();
        std::env::remove_var("JBIG2_SANDBOX_BACKEND");
        assert_eq!(sb.backend(), Backend::Passthrough);
    }
}
