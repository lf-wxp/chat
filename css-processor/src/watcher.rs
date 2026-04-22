//! File watcher module: monitors CSS source directory for changes and triggers rebuild.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result};
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer, notify::RecursiveMode};

use crate::processor;
use crate::sse::Broadcaster;

/// Watch `input_dir` for CSS file changes and rebuild to `output_dir` on each change.
///
/// Uses a 300ms debounce window to batch rapid file system events (e.g., editor
/// save operations that create temp files). Blocks the current thread until
/// an error occurs or the process is interrupted.
///
/// If `dist_dir` is provided, processed CSS is also copied there for live-reload.
/// If `broadcaster` is provided, a `rebuild` SSE event is pushed on every rebuild
/// so browsers can hot-swap CSS without polling.
pub fn watch_and_rebuild(
  input_dir: &Path,
  output_dir: &Path,
  dist_dir: Option<&Path>,
  broadcaster: Option<&Broadcaster>,
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
        // Collect unique CSS paths that actually changed.
        let mut changed: Vec<PathBuf> = events
          .iter()
          .filter(|event| {
            event.kind == DebouncedEventKind::Any
              && event.path.extension().is_some_and(|ext| ext == "css")
          })
          .map(|event| event.path.clone())
          .collect();
        changed.sort();
        changed.dedup();

        if !changed.is_empty() {
          eprintln!(
            "[css-processor] CSS change detected ({} file(s)), rebuilding...",
            changed.len()
          );
          match processor::process_all(input_dir, output_dir) {
            Ok(()) => {
              eprintln!("[css-processor] Rebuild complete.");
              if let Some(dist) = dist_dir
                && let Err(e) = copy_to_dist(output_dir, dist)
              {
                eprintln!("[css-processor] Failed to copy to dist: {e:#}");
              }
              if let Some(b) = broadcaster {
                let payload = build_rebuild_payload(input_dir, &changed);
                b.notify(&payload);
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

/// Build a minimal JSON payload listing the CSS files that changed, relative
/// to `input_dir` and expressed as web paths (forward slashes, `styles/` prefix).
///
/// Both `input_dir` and each `changed` path are canonicalized before stripping
/// so that a relative `input_dir` (e.g. `frontend/styles`) still matches the
/// absolute paths reported by the OS file watcher.
///
/// Example output: `{"files":["styles/main.css","styles/login.css"],"ts":1713...}`
fn build_rebuild_payload(input_dir: &Path, changed: &[PathBuf]) -> String {
  let ts = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis())
    .unwrap_or(0);

  let canonical_root = fs::canonicalize(input_dir).unwrap_or_else(|_| input_dir.to_path_buf());

  let mut files = Vec::with_capacity(changed.len());
  for path in changed {
    let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
    let rel = canonical_path
      .strip_prefix(&canonical_root)
      .unwrap_or(&canonical_path);
    let as_web = rel
      .components()
      .map(|c| c.as_os_str().to_string_lossy().into_owned())
      .collect::<Vec<_>>()
      .join("/");
    // Escape minimal JSON special characters (backslash and double-quote).
    let escaped = as_web.replace('\\', "\\\\").replace('"', "\\\"");
    files.push(format!("\"styles/{escaped}\""));
  }

  format!("{{\"files\":[{}],\"ts\":{ts}}}", files.join(","))
}

/// Copy all files from `output_dir` to `dist_dir`, preserving directory structure.
///
/// This enables CSS live-reload by writing directly to Trunk's dist directory,
/// bypassing Trunk's rebuild cycle entirely.
pub fn copy_to_dist(output_dir: &Path, dist_dir: &Path) -> Result<()> {
  copy_dir_recursive(output_dir, dist_dir)?;
  eprintln!("[css-processor] Copied to {}", dist_dir.display());
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
