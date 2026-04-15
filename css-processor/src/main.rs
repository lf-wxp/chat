//! CSS preprocessor that expands CSS Modules `composes` property.
//!
//! Supports two modes:
//! - **One-shot**: `css-processor <input-dir> <output-dir>`
//! - **Watch**: `css-processor --watch [--dist <dist-dir>] <input-dir> <output-dir>`
//!
//! Uses LightningCSS for parsing and a custom expansion pass for `composes`.

mod processor;
mod watcher;

use std::env;
use std::path::PathBuf;

use anyhow::{Result, bail};

/// Parsed CLI options.
struct CliOptions {
  watch: bool,
  dist_dir: Option<PathBuf>,
  input_dir: PathBuf,
  output_dir: PathBuf,
}

fn main() -> Result<()> {
  let args: Vec<String> = env::args().collect();
  let opts = parse_args(&args)?;

  if !opts.input_dir.exists() {
    bail!("Input directory does not exist: {}", opts.input_dir.display());
  }

  // Always run an initial processing pass
  processor::process_all(&opts.input_dir, &opts.output_dir)?;

  // Copy to dist if specified
  if let Some(ref dist_dir) = opts.dist_dir {
    watcher::copy_to_dist(&opts.output_dir, dist_dir)?;
  }

  if opts.watch {
    eprintln!("[css-processor] Entering watch mode. Press Ctrl+C to stop.");
    watcher::watch_and_rebuild(&opts.input_dir, &opts.output_dir, opts.dist_dir.as_deref())?;
  }

  Ok(())
}

/// Parse CLI arguments into CliOptions.
fn parse_args(args: &[String]) -> Result<CliOptions> {
  let mut watch = false;
  let mut dist_dir: Option<PathBuf> = None;
  let mut positional = Vec::new();
  let mut iter = args.iter().skip(1);

  while let Some(arg) = iter.next() {
    match arg.as_str() {
      "--watch" | "-w" => watch = true,
      "--dist" | "-d" => {
        if let Some(path) = iter.next() {
          dist_dir = Some(PathBuf::from(path));
        } else {
          bail!("--dist requires a directory path argument");
        }
      }
      _ => positional.push(arg.clone()),
    }
  }

  if positional.len() < 2 {
    eprintln!("Usage: css-processor [--watch] [--dist <dist-dir>] <input-dir> <output-dir>");
    eprintln!("  Processes all .css files in <input-dir>, expands `composes`,");
    eprintln!("  and writes results to <output-dir> preserving directory structure.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --watch, -w          Watch <input-dir> for changes and rebuild automatically");
    eprintln!("  --dist, -d <path>    Also copy processed CSS to <path> (for live-reload)");
    std::process::exit(1);
  }

  Ok(CliOptions {
    watch,
    dist_dir,
    input_dir: PathBuf::from(&positional[0]),
    output_dir: PathBuf::from(&positional[1]),
  })
}
