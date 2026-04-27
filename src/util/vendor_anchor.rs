//! Shared helpers for loading and validating vendor pins from
//! `tools/conformance/known-issues.ron`.
//!
//! This module intentionally handles only anchor concerns (`vendor` fields):
//! conformance-matrix-specific row/column validation remains in
//! `tools/conformance/main.rs`.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Anchor type stored in `known-issues.ron`.
pub enum VendorKind {
    /// Submodule git commit pin (`git -C <path> rev-parse HEAD`).
    GitSha,
    /// Direct file-content hash pin (`sha256(path)`).
    FileSha256,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Canonicalized vendor anchor extracted from `known-issues.ron`.
pub struct VendorAnchor {
    /// Relative path from workspace root (for example `vendor/jbig2dec`).
    pub path: PathBuf,
    /// Expected digest/commit hash from catalog.
    pub expected: String,
    /// Anchor kind.
    pub kind: VendorKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Result of checking one anchor against current workspace state.
pub enum AnchorStatus {
    /// Observed value matches expectation.
    Ok {
        /// Current digest/commit hash observed in workspace.
        actual: String,
    },
    /// Observed value differs from expectation.
    Mismatch {
        /// Catalog expectation from `known-issues.ron`.
        expected: String,
        /// Current digest/commit hash observed in workspace.
        actual: String,
    },
}

impl VendorAnchor {
    /// Check this anchor against the current workspace state.
    pub fn check(&self, root: &Path) -> Result<AnchorStatus, String> {
        let actual = match self.kind {
            VendorKind::GitSha => git_head(root, &self.path)?,
            VendorKind::FileSha256 => file_sha256(&root.join(&self.path))?,
        };
        if actual == self.expected {
            Ok(AnchorStatus::Ok { actual })
        } else {
            Ok(AnchorStatus::Mismatch {
                expected: self.expected.clone(),
                actual,
            })
        }
    }

    /// Find a `GitSha` anchor whose vendor path contains this binary.
    pub fn lookup_for_binary<'a>(
        anchors: &'a [Self],
        binary: &Path,
        root: &Path,
    ) -> Option<&'a Self> {
        let rel = binary.strip_prefix(root).ok()?;
        anchors
            .iter()
            .filter(|a| matches!(a.kind, VendorKind::GitSha))
            .find(|a| rel == a.path || rel.starts_with(&a.path))
    }

    /// Return the expected digest shortened for log banners.
    pub fn short_expected(&self) -> String {
        short_sha(&self.expected)
    }
}

/// Load and deduplicate vendor anchors from `known-issues.ron`.
pub fn load_anchors(root: &Path) -> Result<Vec<VendorAnchor>, String> {
    let path = known_issues_path(root);
    let text =
        fs::read_to_string(&path).map_err(|err| format!("read known issues {:?}: {err}", path))?;
    let entries: Vec<RawIssue> =
        ron::from_str(&text).map_err(|err| format!("parse known issues {:?}: {err}", path))?;

    let mut dedup: BTreeMap<AnchorKey, (String, usize)> = BTreeMap::new();
    for (idx, entry) in entries.iter().enumerate() {
        let Some(vendor) = entry.vendor.as_ref() else {
            continue;
        };
        let (kind, anchor_path, expected) = match vendor {
            RawVendor::GitSha { path, sha } => (VendorKind::GitSha, path.clone(), sha.clone()),
            RawVendor::FileSha256 { path, sha256 } => {
                (VendorKind::FileSha256, path.clone(), sha256.clone())
            }
        };
        let key = AnchorKey {
            kind,
            path: anchor_path.clone(),
        };
        if let Some((prev, prev_idx)) = dedup.get(&key) {
            if prev != &expected {
                return Err(format!(
                    "known-issues vendor pin conflict for {} {:?}: entry #{} has {}, entry #{} has {}",
                    anchor_path, kind, prev_idx, prev, idx, expected
                ));
            }
            continue;
        }
        dedup.insert(key, (expected, idx));
    }

    Ok(dedup
        .into_iter()
        .map(|(key, (expected, _))| VendorAnchor {
            path: PathBuf::from(key.path),
            expected,
            kind: key.kind,
        })
        .collect())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct AnchorKey {
    kind: VendorKind,
    path: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RawIssue {
    vendor: Option<RawVendor>,
}

#[derive(Clone, Debug, Deserialize)]
enum RawVendor {
    GitSha { path: String, sha: String },
    FileSha256 { path: String, sha256: String },
}

fn known_issues_path(root: &Path) -> PathBuf {
    root.join("tools")
        .join("conformance")
        .join("known-issues.ron")
}

fn git_head(root: &Path, rel_path: &Path) -> Result<String, String> {
    let path = root.join(rel_path);
    let output = Command::new("git")
        .arg("-C")
        .arg(&path)
        .arg("rev-parse")
        .arg("HEAD")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("spawn git for {}: {err}", rel_path.display()))?;
    if !output.status.success() {
        return Err(format!(
            "git rev-parse failed for {}: {}",
            rel_path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let output = Command::new("shasum")
        .arg("-a")
        .arg("256")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .or_else(|_| {
            Command::new("sha256sum")
                .arg(path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        })
        .map_err(|err| format!("spawn sha256 tool for {path:?}: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "hash command failed for {path:?}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .split_whitespace()
        .next()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("hash command produced no digest for {path:?}"))
}

/// Render a digest with a stable 12-character prefix.
pub fn short_sha(sha: &str) -> String {
    sha.chars().take(12).collect()
}

impl fmt::Display for VendorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitSha => write!(f, "GitSha"),
            Self::FileSha256 => write!(f, "FileSha256"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock drift")
            .as_nanos();
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp-tests");
        fs::create_dir_all(&base).expect("create tmp-tests dir");
        let dir = base.join(format!("jbig2-{prefix}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn write_known_issues(root: &Path, body: &str) {
        let dir = root.join("tools").join("conformance");
        fs::create_dir_all(&dir).expect("create known issues dir");
        fs::write(dir.join("known-issues.ron"), body).expect("write known issues");
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let template_dir = dir.join(".git-template");
        fs::create_dir_all(&template_dir).expect("create git template dir");
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .env("GIT_TEMPLATE_DIR", &template_dir)
            .status()
            .expect("spawn git");
        assert!(
            status.success(),
            "git {:?} failed in {}",
            args,
            dir.display()
        );
    }

    fn git_head_for(dir: &Path) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .expect("git rev-parse");
        assert!(out.status.success());
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[test]
    fn load_anchors_dedups_repeated_pins() {
        let root = unique_temp_dir("anchor-dedup");
        write_known_issues(
            &root,
            r#"
[
    (vendor: Some(GitSha(path: "vendor/jbig2dec", sha: "abc"))),
    (vendor: Some(GitSha(path: "vendor/jbig2dec", sha: "abc"))),
]
"#,
        );
        let anchors = load_anchors(&root).expect("load anchors");
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].path, PathBuf::from("vendor/jbig2dec"));
        assert_eq!(anchors[0].expected, "abc");
        assert_eq!(anchors[0].kind, VendorKind::GitSha);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_anchors_rejects_conflicting_pins() {
        let root = unique_temp_dir("anchor-conflict");
        write_known_issues(
            &root,
            r#"
[
    (vendor: Some(GitSha(path: "vendor/jbig2dec", sha: "abc"))),
    (vendor: Some(GitSha(path: "vendor/jbig2dec", sha: "def"))),
]
"#,
        );
        let err = load_anchors(&root).expect_err("expected conflict");
        assert!(err.contains("entry #0"), "{err}");
        assert!(err.contains("entry #1"), "{err}");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn check_ok_and_mismatch() {
        let root = unique_temp_dir("anchor-check");
        let repo = root.join("vendor").join("jbig2dec");
        fs::create_dir_all(&repo).expect("create repo");
        run_git(&repo, &["init", "."]);
        fs::write(repo.join("README"), "seed\n").expect("write file");
        run_git(&repo, &["add", "README"]);
        run_git(&repo, &["commit", "-m", "init"]);
        let actual = git_head_for(&repo);

        let anchor = VendorAnchor {
            path: PathBuf::from("vendor/jbig2dec"),
            expected: actual.clone(),
            kind: VendorKind::GitSha,
        };
        assert_eq!(
            anchor.check(&root).expect("check"),
            AnchorStatus::Ok {
                actual: actual.clone()
            }
        );

        let mismatch = VendorAnchor {
            expected: "deadbeef".to_string(),
            ..anchor
        };
        let status = mismatch.check(&root).expect("check mismatch");
        assert!(
            matches!(status, AnchorStatus::Mismatch { expected, actual: _ } if expected == "deadbeef")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn lookup_for_binary_matches_vendor_prefix() {
        let _guard = env_lock().lock().expect("env lock");
        let root = unique_temp_dir("anchor-lookup");
        let anchors = vec![VendorAnchor {
            path: PathBuf::from("vendor/jbig2dec"),
            expected: "abc".to_string(),
            kind: VendorKind::GitSha,
        }];
        let hit = VendorAnchor::lookup_for_binary(
            &anchors,
            &root.join("vendor/jbig2dec/jbig2dec"),
            &root,
        );
        assert!(hit.is_some());
        let miss = VendorAnchor::lookup_for_binary(
            &anchors,
            &root.join("target/release/jbig2-decode"),
            &root,
        );
        assert!(miss.is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_anchors_supports_file_sha256() {
        let root = unique_temp_dir("anchor-filesha");
        let sample = root.join("vendor").join("sample.bin");
        fs::create_dir_all(sample.parent().expect("parent")).expect("dir");
        let mut f = fs::File::create(&sample).expect("create sample");
        writeln!(f, "hello").expect("write");
        let digest = file_sha256(&sample).expect("hash");
        write_known_issues(
            &root,
            &format!(
                r#"
[
    (vendor: Some(FileSha256(path: "vendor/sample.bin", sha256: "{digest}"))),
]
"#
            ),
        );
        let anchors = load_anchors(&root).expect("load");
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].kind, VendorKind::FileSha256);
        let _ = fs::remove_dir_all(root);
    }
}
