//! Core CSS processing logic: file collection, parsing, and `composes` expansion.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};

/// Process all CSS files from `input_dir` and write expanded output to `output_dir`.
pub fn process_all(input_dir: &Path, output_dir: &Path) -> Result<()> {
  let css_files = collect_css_files(input_dir)?;
  eprintln!(
    "[css-processor] Found {} CSS files in {}",
    css_files.len(),
    input_dir.display()
  );

  for css_file in &css_files {
    let relative = css_file
      .strip_prefix(input_dir)
      .context("Failed to compute relative path")?;
    let output_path = output_dir.join(relative);

    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let input_css =
      fs::read_to_string(css_file).with_context(|| format!("Failed to read: {}", css_file.display()))?;

    let output_css = process_css(&input_css, css_file)?;

    fs::write(&output_path, &output_css)
      .with_context(|| format!("Failed to write: {}", output_path.display()))?;

    eprintln!(
      "[css-processor] Processed: {}",
      relative.display()
    );
  }

  eprintln!(
    "[css-processor] Done. Output written to {}",
    output_dir.display()
  );
  Ok(())
}

/// Recursively collect all `.css` files in a directory.
fn collect_css_files(dir: &Path) -> Result<Vec<PathBuf>> {
  let mut files = Vec::new();
  collect_css_files_recursive(dir, &mut files)?;
  files.sort();
  Ok(files)
}

fn collect_css_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
  for entry in
    fs::read_dir(dir).with_context(|| format!("Failed to read directory: {}", dir.display()))?
  {
    let entry = entry?;
    let path = entry.path();
    if path.is_dir() {
      collect_css_files_recursive(&path, files)?;
    } else if path.extension().is_some_and(|ext| ext == "css") {
      files.push(path);
    }
  }
  Ok(())
}

/// Process a single CSS file: parse with LightningCSS, expand `composes`, and return output.
fn process_css(input: &str, filename: &Path) -> Result<String> {
  let filename_str = filename.to_string_lossy().to_string();

  let options = ParserOptions {
    filename: filename_str.clone(),
    ..ParserOptions::default()
  };

  match StyleSheet::parse(input, options) {
    Ok(stylesheet) => {
      let printer_options = PrinterOptions {
        minify: false,
        ..PrinterOptions::default()
      };
      match stylesheet.to_css(printer_options) {
        Ok(result) => Ok(expand_composes(&result.code)),
        Err(_) => Ok(expand_composes(input)),
      }
    }
    Err(_) => {
      // LightningCSS cannot parse (e.g., uses @scope, @starting-style, etc.)
      // Fall back to composes expansion on raw input
      Ok(expand_composes(input))
    }
  }
}

/// Expand `composes` declarations in CSS text.
///
/// Two-pass approach:
/// 1. Extract all class selectors and their top-level declarations
/// 2. Replace each `composes: <class-name>;` with the referenced class's declarations
pub fn expand_composes(css: &str) -> String {
  let class_declarations = extract_class_declarations(css);

  let mut result = String::with_capacity(css.len());
  let chars: Vec<char> = css.chars().collect();
  let len = chars.len();
  let mut i = 0;

  while i < len {
    if i + 9 <= len && &css[byte_pos(css, i)..byte_pos(css, i + 9)] == "composes:" {
      let start = i;
      let mut end = i;
      while end < len && chars[end] != ';' {
        end += 1;
      }
      if end < len {
        end += 1;
      }

      let composes_text = &css[byte_pos(css, start + 9)..byte_pos(css, end - 1)];
      let composes_value = composes_text.trim();
      let composed_classes: Vec<&str> = composes_value.split_whitespace().collect();

      for class_name in &composed_classes {
        let lookup_name = class_name.trim_start_matches('.');
        if let Some(declarations) = class_declarations.get(lookup_name) {
          for decl in declarations {
            result.push_str("  ");
            result.push_str(decl);
            result.push('\n');
          }
        }
      }

      i = end;
      while i < len && (chars[i] == '\n' || chars[i] == '\r') {
        i += 1;
      }
    } else {
      result.push(chars[i]);
      i += 1;
    }
  }

  result
}

/// Get byte position from char position in a string.
fn byte_pos(s: &str, char_pos: usize) -> usize {
  s.char_indices()
    .nth(char_pos)
    .map(|(byte_idx, _)| byte_idx)
    .unwrap_or(s.len())
}

/// Extract declarations for each class selector.
///
/// Returns a map from class name (without `.`) to a list of CSS declarations.
/// Only handles simple `.class-name { ... }` rules (single class selectors).
pub fn extract_class_declarations(css: &str) -> HashMap<String, Vec<String>> {
  let mut map: HashMap<String, Vec<String>> = HashMap::new();
  let chars: Vec<char> = css.chars().collect();
  let len = chars.len();
  let mut i = 0;

  while i < len {
    // Skip comments
    if i + 1 < len && chars[i] == '/' && chars[i + 1] == '*' {
      i += 2;
      while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
        i += 1;
      }
      i += 2;
      continue;
    }

    // Look for `.class-name {`
    if chars[i] == '.' {
      let class_start = i + 1;
      let mut j = class_start;

      while j < len && (chars[j].is_alphanumeric() || chars[j] == '-' || chars[j] == '_') {
        j += 1;
      }

      if j > class_start {
        let class_name: String = chars[class_start..j].iter().collect();

        let mut k = j;
        while k < len && chars[k].is_whitespace() {
          k += 1;
        }

        if k < len && chars[k] == '{' {
          let block_start = k + 1;
          let mut depth = 1;
          let mut block_end = block_start;

          while block_end < len && depth > 0 {
            if chars[block_end] == '{' {
              depth += 1;
            } else if chars[block_end] == '}' {
              depth -= 1;
            }
            if depth > 0 {
              block_end += 1;
            }
          }

          let block_content: String = chars[block_start..block_end].iter().collect();
          let declarations = parse_top_level_declarations(&block_content);

          let filtered: Vec<String> = declarations
            .into_iter()
            .filter(|d| !d.trim_start().starts_with("composes:"))
            .collect();

          if !filtered.is_empty() {
            map.insert(class_name, filtered);
          }

          i = block_end + 1;
          continue;
        }
      }
    }

    i += 1;
  }

  map
}

/// Parse top-level CSS declarations from a block (ignoring nested blocks).
pub fn parse_top_level_declarations(block: &str) -> Vec<String> {
  let mut declarations = Vec::new();
  let chars: Vec<char> = block.chars().collect();
  let len = chars.len();
  let mut i = 0;
  let mut depth = 0;
  let mut current_decl = String::new();

  while i < len {
    let ch = chars[i];

    if ch == '{' {
      depth += 1;
      i += 1;
      while i < len && depth > 0 {
        if chars[i] == '{' {
          depth += 1;
        } else if chars[i] == '}' {
          depth -= 1;
        }
        i += 1;
      }
      current_decl.clear();
      continue;
    }

    if ch == ';' && depth == 0 {
      current_decl.push(';');
      let trimmed = current_decl.trim().to_string();
      if !trimmed.is_empty() && trimmed != ";" {
        declarations.push(trimmed);
      }
      current_decl.clear();
    } else {
      current_decl.push(ch);
    }

    i += 1;
  }

  declarations
}

#[cfg(test)]
mod tests;
