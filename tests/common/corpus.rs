//! Shared helpers for the checked-in patent TIFF benchmark corpus.
#![allow(dead_code)]

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use jbig2::Bitmap;

const MANIFEST_NAME: &str = "manifest.tsv";
const THRESHOLD: u8 = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SizeBucket {
    Tiny,
    Small,
    Medium,
    Large,
}

impl SizeBucket {
    pub const ALL: [Self; 4] = [Self::Tiny, Self::Small, Self::Medium, Self::Large];

    pub fn label(self) -> &'static str {
        match self {
            Self::Tiny => "tiny",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "tiny" => Some(Self::Tiny),
            "small" => Some(Self::Small),
            "medium" => Some(Self::Medium),
            "large" => Some(Self::Large),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CorpusEntry {
    pub source_path: PathBuf,
    pub copied_path: PathBuf,
    pub compressed_bytes: u64,
    pub bucket: SizeBucket,
}

pub fn source_corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("patent-samples")
}

pub fn repo_corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benchmark-corpus")
        .join("patent-10k")
}

pub fn manifest_path() -> PathBuf {
    repo_corpus_dir().join(MANIFEST_NAME)
}

pub fn ensure_repo_corpus() -> Result<Vec<CorpusEntry>, String> {
    if !manifest_path().is_file() {
        return Err(format!(
            "checked-in benchmark corpus is missing at {}.\n\
             expected the manifest to be present in this checkout.\n\
             if you are intentionally re-rolling the fixed sample (maintainer only), run:\n\
             \n\
                 CARGO_HOME=./.cargo cargo run --release --example prepare_corpus -- --confirm\n",
            manifest_path().display(),
        ));
    }
    read_manifest()
}

pub fn manifest_subset(n: usize) -> Result<Vec<CorpusEntry>, String> {
    let mut entries = ensure_repo_corpus()?;
    if entries.len() > n {
        entries.truncate(n);
    }
    Ok(entries)
}

pub fn bucket_entries(bucket: SizeBucket, max: usize) -> Result<Vec<CorpusEntry>, String> {
    let entries = ensure_repo_corpus()?;
    Ok(entries
        .into_iter()
        .filter(|entry| entry.bucket == bucket)
        .take(max)
        .collect())
}

pub fn load_bucket(bucket: SizeBucket, max: usize) -> Result<Vec<(CorpusEntry, Bitmap)>, String> {
    let entries = bucket_entries(bucket, max)?;
    entries
        .into_iter()
        .map(|entry| {
            let bitmap = load_bitmap(&entry.copied_path)?;
            Ok((entry, bitmap))
        })
        .collect()
}

pub fn load_bitmap(path: &Path) -> Result<Bitmap, String> {
    let gray = image::open(path)
        .map_err(|e| format!("open {path:?}: {e}"))?
        .into_luma8();
    let (width, height) = gray.dimensions();
    let mut bitmap = Bitmap::new(width, height).map_err(|e| e.to_string())?;
    let width_usize = width as usize;
    let tail_bits = (width & 7) as u8;
    let tail_mask = if tail_bits == 0 {
        0xFF
    } else {
        0xFFu8 << (8 - tail_bits)
    };

    for y in 0..height as usize {
        let src = &gray.as_raw()[y * width_usize..(y + 1) * width_usize];
        let row = bitmap.row_mut(y);
        for (x, &pix) in src.iter().enumerate() {
            if pix < THRESHOLD {
                row[x >> 3] |= 1u8 << (7 - (x & 7));
            }
        }
        if tail_mask != 0xFF {
            let last = row.len() - 1;
            row[last] &= tail_mask;
        }
    }

    Ok(bitmap)
}

fn read_manifest() -> Result<Vec<CorpusEntry>, String> {
    let file =
        File::open(manifest_path()).map_err(|e| format!("open manifest {:?}: {e}", manifest_path()))?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for (line_no, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| {
            format!(
                "read manifest line {} {:?}: {e}",
                line_no + 1,
                manifest_path()
            )
        })?;
        let mut parts = line.splitn(4, '\t');
        let source_path = parts
            .next()
            .ok_or_else(|| format!("manifest line {} missing source path", line_no + 1))?;
        let copied_rel = parts
            .next()
            .ok_or_else(|| format!("manifest line {} missing copied path", line_no + 1))?;
        let compressed_bytes = parts
            .next()
            .ok_or_else(|| format!("manifest line {} missing compressed bytes", line_no + 1))?;
        let bucket = parts
            .next()
            .ok_or_else(|| format!("manifest line {} missing bucket", line_no + 1))?;
        let bucket = SizeBucket::parse(bucket).ok_or_else(|| {
            format!(
                "manifest line {} has unknown bucket {bucket:?}",
                line_no + 1
            )
        })?;
        entries.push(CorpusEntry {
            source_path: PathBuf::from(source_path),
            copied_path: repo_corpus_dir().join(copied_rel),
            compressed_bytes: compressed_bytes.parse().map_err(|e| {
                format!(
                    "manifest line {} invalid compressed bytes {:?}: {e}",
                    line_no + 1,
                    compressed_bytes
                )
            })?,
            bucket,
        });
    }
    Ok(entries)
}
