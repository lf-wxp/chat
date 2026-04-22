//! CSS preprocessor that expands CSS Modules `composes` property.
//!
//! Supports two modes:
//! - **One-shot**: `css-processor <input-dir> <output-dir>`
//! - **Watch**: `css-processor --watch [--dist <dist-dir>] [--sse-port <port>] <input-dir> <output-dir>`
//!
//! Uses LightningCSS for parsing and a custom expansion pass for `composes`.
//! In watch mode, an optional SSE server pushes `rebuild` events to browsers
//! so the dev page can hot-swap CSS without polling.

mod processor;
mod sse;
mod watcher;

use std::env;
use std::path::PathBuf;

use anyhow::{Result, bail};

/// Default port for the SSE hot-reload server.
const DEFAULT_SSE_PORT: u16 = 8765;

/// Parsed CLI options.
struct CliOptions {
  watch: bool,
  dist_dir: Option<PathBuf>,
  input_dir: PathBuf,
  output_dir: PathBuf,
  sse_port: u16,
  sse_disabled: bool,
}

fn main() -> Result<()> {
  let args: Vec<String> = env::args().collect();
  let opts = parse_args(&args)?;

  if !opts.input_dir.exists() {
    bail!(
      "Input directory does not exist: {}",
      opts.input_dir.display()
    );
  }

  // Always run an initial processing pass
  processor::process_all(&opts.input_dir, &opts.output_dir)?;

  // Copy to dist if specified
  if let Some(ref dist_dir) = opts.dist_dir {
    watcher::copy_to_dist(&opts.output_dir, dist_dir)?;
  }

  if opts.watch {
    eprintln!("[css-processor] Entering watch mode. Press Ctrl+C to stop.");

    // Start SSE broadcaster (unless explicitly disabled).
    let broadcaster = if opts.sse_disabled {
      None
    } else {
      match sse::Broadcaster::start(opts.sse_port) {
        Ok(b) => Some(b),
        Err(e) => {
          eprintln!(
            "[css-processor] Warning: SSE server failed to start ({e:#}); hot-reload will be unavailable"
          );
          None
        }
      }
    };

    watcher::watch_and_rebuild(
      &opts.input_dir,
      &opts.output_dir,
      opts.dist_dir.as_deref(),
      broadcaster.as_ref(),
    )?;
  }

  Ok(())
}

/// Parse CLI arguments into CliOptions.
fn parse_args(args: &[String]) -> Result<CliOptions> {
  let mut watch = false;
  let mut dist_dir: Option<PathBuf> = None;
  let mut sse_port: u16 = DEFAULT_SSE_PORT;
  let mut sse_disabled = false;
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
      "--sse-port" => {
        if let Some(port_str) = iter.next() {
          sse_port = port_str
            .parse()
            .map_err(|_| anyhow::anyhow!("--sse-port requires a valid u16 port number"))?;
        } else {
          bail!("--sse-port requires a port number argument");
        }
      }
      "--no-sse" => sse_disabled = true,
      _ => positional.push(arg.clone()),
    }
  }

  if positional.len() < 2 {
    eprintln!(
      "Usage: css-processor [--watch] [--dist <dist-dir>] [--sse-port <port>] [--no-sse] <input-dir> <output-dir>"
    );
    eprintln!("  Processes all .css files in <input-dir>, expands `composes`,");
    eprintln!("  and writes results to <output-dir> preserving directory structure.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --watch, -w            Watch <input-dir> for changes and rebuild automatically");
    eprintln!("  --dist, -d <path>      Also copy processed CSS to <path> (for live-reload)");
    eprintln!(
      "  --sse-port <port>      Port for SSE hot-reload server (default: {DEFAULT_SSE_PORT})"
    );
    eprintln!("  --no-sse               Disable the SSE hot-reload server");
    std::process::exit(1);
  }

  Ok(CliOptions {
    watch,
    dist_dir,
    input_dir: PathBuf::from(&positional[0]),
    output_dir: PathBuf::from(&positional[1]),
    sse_port,
    sse_disabled,
  })
}
