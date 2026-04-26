//! `tools/jbig2-decode`: minimal sandboxable child binary that runs
//! `jbig2::Jbig2Decoder` against a single stream.
//!
//! The harness in `tools/corpus-validator` invokes this binary inside the
//! same `Sandbox::for_decoder()` it uses for `jbig2dec` and the ITU sample
//! decoder. Doing it that way means hostile-input behaviour of the rust
//! decoder (panics, OOM, runaway loops) cannot take down the harness, and
//! the resulting `SandboxOutcome` is classified by the same code path as
//! every other external decoder we run.
//!
//! Exit codes are deliberately stable so the harness can score the rust
//! column without parsing stderr:
//!
//! - `0` — every page decoded without error.
//! - `1` — `Jbig2Decoder::new` or `decode_page` returned `Err`. The harness
//!   classifies this as `RejectErr`.
//! - `2` — bad invocation (missing argv, unreadable file). Distinct from
//!   `1` so harness logs surface harness bugs rather than decoder bugs.
//!
//! Panics are converted to `SIGABRT` via `std::panic::set_hook` +
//! `std::process::abort`. The sandbox sees the resulting signal and the
//! harness classifies it as `Crash`. This is the whole reason the harness
//! cannot just call `Jbig2Decoder` in-process: in-process panics could
//! abort the harness itself, and `catch_unwind` does not protect against
//! aborts, OOMs, or stack overflows on hostile input.
//!
//! The binary deliberately does not link the validator. Validation runs
//! in-process inside `corpus-validator` per fixture; this binary's only job
//! is decoder behaviour observation under the sandbox.

use std::io::{Cursor, Write};
use std::process::ExitCode;

const EXIT_OK: u8 = 0;
const EXIT_DECODE_ERROR: u8 = 1;
const EXIT_BAD_ARGS: u8 = 2;

fn main() -> ExitCode {
    install_abort_panic_hook();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        let _ = writeln!(
            std::io::stderr(),
            "jbig2-decode: usage: {} <stream-path>",
            args.first().map(String::as_str).unwrap_or("jbig2-decode"),
        );
        return ExitCode::from(EXIT_BAD_ARGS);
    }

    let path = &args[1];
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(err) => {
            let _ = writeln!(std::io::stderr(), "jbig2-decode: read {path}: {err}");
            return ExitCode::from(EXIT_BAD_ARGS);
        }
    };

    match decode(&bytes) {
        Ok(pages) => {
            let _ = writeln!(std::io::stdout(), "jbig2-decode: ok pages={pages}");
            ExitCode::from(EXIT_OK)
        }
        Err(msg) => {
            let _ = writeln!(std::io::stderr(), "jbig2-decode: error: {msg}");
            ExitCode::from(EXIT_DECODE_ERROR)
        }
    }
}

fn decode(bytes: &[u8]) -> Result<u32, String> {
    let mut decoder = jbig2::Jbig2Decoder::new(Cursor::new(bytes))
        .map_err(|err| format!("Jbig2Decoder::new failed: {err}"))?;
    let pages = decoder.num_pages();
    if pages == 0 {
        // Empty page set is a valid successful outcome. The harness wants
        // the exit code to differentiate "decoder said the file was OK"
        // from "decoder said the file was bad", and an embedded fragment
        // with no pages falls in the former.
        return Ok(0);
    }
    let mut decoded = 0u32;
    for page in 1..=pages {
        decoder
            .decode_page(page)
            .map_err(|err| format!("decode_page({page}) failed: {err}"))?;
        decoded += 1;
    }
    Ok(decoded)
}

/// Convert any subsequent panic into a `SIGABRT` so the sandbox sees a
/// signal-terminated child rather than a panicking one. We deliberately do
/// NOT use `panic = "abort"` at the profile level because that would also
/// affect the rest of the workspace; setting the hook here keeps the
/// behaviour binary-local.
fn install_abort_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|loc| format!("{}:{}", loc.file(), loc.line()))
            .unwrap_or_else(|| "<unknown>".into());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("<no payload>");
        let _ = writeln!(
            std::io::stderr(),
            "jbig2-decode: panic at {location}: {payload}"
        );
        std::process::abort();
    }));
}
