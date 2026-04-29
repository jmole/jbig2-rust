use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, ValueEnum};
use jbig2::validator::{validate, Lens};

#[derive(Parser)]
#[command(name = "jbig2-validate")]
#[command(about = "Validate a JBIG2 stream against T.88 structural rules")]
struct Args {
    /// Input JBIG2 file.
    input: PathBuf,
    /// Conformance lens.
    #[arg(long, default_value = "strict-t88")]
    lens: CliLens,
    /// Emit JSON instead of text.
    #[arg(long)]
    json: bool,
    /// Output format (text or json). Equivalent to --json when set to json.
    #[arg(long, value_enum, default_value = "text")]
    format: CliFormat,
}

#[derive(Clone, Copy, ValueEnum, PartialEq, Eq)]
enum CliFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum CliLens {
    StrictT88,
    Jbig2decInterop,
    ItuT88Interop,
    ImageioInterop,
}

impl From<CliLens> for Lens {
    fn from(value: CliLens) -> Self {
        match value {
            CliLens::StrictT88 => Self::StrictT88,
            CliLens::Jbig2decInterop => Self::Jbig2decInterop,
            CliLens::ItuT88Interop => Self::ItuT88Interop,
            CliLens::ImageioInterop => Self::ImageioInterop,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let bytes = fs::read(&args.input)
        .with_context(|| format!("failed to read {}", args.input.display()))?;
    let report = validate(&bytes, args.lens.into());
    let want_json = args.json || args.format == CliFormat::Json;
    if want_json {
        println!("{}", report.render_json());
    } else {
        print!("{}", report.render_text());
    }
    if report.is_invalid() {
        std::process::exit(1);
    }
    Ok(())
}
