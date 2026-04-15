//! File watcher module: monitors CSS source directory for changes and triggers rebuild.

use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result};
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer, notify::RecursiveMode};

use crate::processor;

/// Watch `input_dir` for CSS file changes and rebuild to `output_dir` on each change.
///
/// Uses a 300ms debounce window to batch rapid file system events (e.g., editor
/// save operations that create temp files). Blocks the current thread until
/// an error occurs or the process is interrupted.
///
/// If `dist_dir` is provided, processed CSS is also copied there for live-reload.
pub fn watch_and_rebuild(
  input_dir: &Path,
  output_dir: &Path,
  dist_dir: Option<&Path>,
) -> Result<()> {
  let (tx, rx) = mpsc::channel();

  let mut debouncer =
    new_debouncer(Duration::from_millis(300), tx).context("Failed to create file watcher")?;

  debouncer
    .watcher()
    .watch(input_dir, RecursiveMode::Recursive)
    .with_context(|| format!("Failed to watch directory: {}", input_dir.display()))?;

  eprintln!(
    "[css-processor] Watching {} for changes...",
    input_dir.display()
  );

  loop {
    match rx.recv() {
      Ok(Ok(events)) => {
        let has_css_change = events.iter().any(|event| {
          event.kind == DebouncedEventKind::Any
            && event.path.extension().is_some_and(|ext| ext == "css")
        });

        if has_css_change {
          eprintln!("[css-processor] CSS change detected, rebuilding...");
          match processor::process_all(input_dir, output_dir) {
            Ok(()) => {
              eprintln!("[css-processor] Rebuild complete.");
              if let Some(dist) = dist_dir
                && let Err(e) = copy_to_dist(output_dir, dist)
              {
                eprintln!("[css-processor] Failed to copy to dist: {e:#}");
              }
            }
            Err(e) => eprintln!("[css-processor] Rebuild failed: {e:#}"),
          }
        }
      }
      Ok(Err(errors)) => {
        eprintln!("[css-processor] Watch error: {errors:?}");
      }
      Err(e) => {
        eprintln!("[css-processor] Watch channel closed: {e}");
        break;
      }
    }
  }

  Ok(())
}

/// Copy all files from `output_dir` to `dist_dir`, preserving directory structure.
///
/// This enables CSS live-reload by writing directly to Trunk's dist directory,
/// bypassing Trunk's rebuild cycle entirely.
pub fn copy_to_dist(output_dir: &Path, dist_dir: &Path) -> Result<()> {
  copy_dir_recursive(output_dir, dist_dir)?;
  eprintln!(
    "[css-processor] Copied to {}",
    dist_dir.display()
  );
  Ok(())
}

/// Recursively copy a directory tree.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
  fs::create_dir_all(dst)
    .with_context(|| format!("Failed to create directory: {}", dst.display()))?;

  for entry in
    fs::read_dir(src).with_context(|| format!("Failed to read directory: {}", src.display()))?
  {
    let entry = entry?;
    let src_path = entry.path();
    let dst_path = dst.join(entry.file_name());

    if src_path.is_dir() {
      copy_dir_recursive(&src_path, &dst_path)?;
    } else {
      fs::copy(&src_path, &dst_path).with_context(|| {
        format!(
          "Failed to copy {} -> {}",
          src_path.display(),
          dst_path.display()
        )
      })?;
    }
  }

  Ok(())
}
