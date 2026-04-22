//! `jbig2` — command-line driver for the `jbig2` crate.
//!
//! Subcommands:
//!
//! * `jbig2 decode INPUT.jb2 OUTPUT.png`
//! * `jbig2 info   INPUT.jb2`
//! * `jbig2 encode INPUT.png OUTPUT.jb2 [--preset fast|balanced|max]`

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, Context};
use clap::{Parser, Subcommand, ValueEnum};

use jbig2::bitmap::Bitmap;
use jbig2::encoder::{Coding, EncoderConfig, GenericTemplate, Jbig2Encoder, Mode};
use jbig2::Jbig2Decoder;

#[derive(Parser)]
#[command(name = "jbig2", version, about = "JBIG2 encoder/decoder CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Decode a JBIG2 file and write the first page as a PNG.
    Decode {
        /// Input `.jb2` file.
        input: PathBuf,
        /// Output PNG file.
        output: PathBuf,
        /// 1-based page index to decode (defaults to 1).
        #[arg(long, default_value_t = 1)]
        page: u32,
    },
    /// Print summary information about a JBIG2 file.
    Info {
        /// Input `.jb2` file.
        input: PathBuf,
    },
    /// Encode a 1-bit or grayscale image as JBIG2.
    Encode {
        /// Input image file (any format supported by `image`).
        input: PathBuf,
        /// Output `.jb2` file.
        output: PathBuf,
        /// Compression preset.
        #[arg(long, value_enum, default_value_t = Preset::Balanced)]
        preset: Preset,
        /// Threshold value (0..=255) used to binarise grayscale input.
        /// Pixels >= threshold are treated as background.
        #[arg(long, default_value_t = 128)]
        threshold: u8,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Preset {
    Fast,
    Balanced,
    Max,
}

impl Preset {
    fn config(self) -> EncoderConfig {
        match self {
            Self::Fast => EncoderConfig::fast(),
            Self::Balanced => EncoderConfig::balanced(),
            Self::Max => EncoderConfig::max_compression(),
        }
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Decode { input, output, page } => cmd_decode(input, output, page),
        Cmd::Info { input } => cmd_info(input),
        Cmd::Encode {
            input,
            output,
            preset,
            threshold,
        } => cmd_encode(input, output, preset, threshold),
    }
}

fn cmd_decode(input: PathBuf, output: PathBuf, page: u32) -> anyhow::Result<()> {
    let file = File::open(&input).with_context(|| format!("open {input:?}"))?;
    let mut dec = Jbig2Decoder::new(BufReader::new(file))?;
    let decoded = dec.decode_page(page)?;
    let bm = decoded.bitmap;
    let mut img = image::GrayImage::new(bm.width(), bm.height());
    for y in 0..bm.height() {
        for x in 0..bm.width() {
            let bit = bm.get_pixel(x as i32, y as i32);
            img.put_pixel(x, y, image::Luma([if bit == 0 { 255 } else { 0 }]));
        }
    }
    img.save(&output)
        .with_context(|| format!("write {output:?}"))?;
    eprintln!("decoded page {} ({}×{}) to {:?}", page, bm.width(), bm.height(), output);
    Ok(())
}

fn cmd_info(input: PathBuf) -> anyhow::Result<()> {
    let file = File::open(&input).with_context(|| format!("open {input:?}"))?;
    let dec = Jbig2Decoder::new(BufReader::new(file))?;
    let fh = dec.file_header();
    println!("file header:");
    println!("  sequential:           {}", fh.sequential);
    println!("  unknown_page_count:   {}", fh.unknown_page_count);
    println!("  uses_extended_template: {}", fh.uses_extended_template);
    println!("  uses_colour:          {}", fh.uses_colour);
    match fh.num_pages {
        Some(n) => println!("  num_pages:            {n}"),
        None => println!("  num_pages:            (unknown)"),
    }
    println!("segments:");
    for (i, sh) in dec.segment_headers().enumerate() {
        println!(
            "  [{i:>3}] #{:<5} type={:<40} page={} len={}",
            sh.number,
            format!("{:?}", sh.segment_type),
            sh.page_association,
            sh.data_length
                .map(|n| n.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        );
    }
    Ok(())
}

fn cmd_encode(
    input: PathBuf,
    output: PathBuf,
    preset: Preset,
    threshold: u8,
) -> anyhow::Result<()> {
    let img = image::open(&input).with_context(|| format!("open {input:?}"))?;
    let gray = img.into_luma8();
    let (w, h) = gray.dimensions();
    let mut bm = Bitmap::new(w, h).map_err(|e| anyhow!("{e}"))?;
    for y in 0..h {
        for x in 0..w {
            let pix = gray.get_pixel(x, y).0[0];
            if pix < threshold {
                bm.set_pixel(x as i32, y as i32, 1);
            }
        }
    }
    let file = File::create(&output).with_context(|| format!("create {output:?}"))?;
    let mut enc = Jbig2Encoder::new(BufWriter::new(file), preset.config());
    enc.write_page(&bm).map_err(|e| anyhow!("{e}"))?;
    enc.finish().map_err(|e| anyhow!("{e}"))?;
    eprintln!(
        "encoded {}×{} with preset={:?} into {:?}",
        w, h, preset, output
    );
    let _ = (GenericTemplate::T0, Coding::Arithmetic, Mode::Generic); // keep `use`s
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
