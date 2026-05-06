//! Subtitle file parsers for Theater mode (Req 12.4a).
//!
//! Supports two formats:
//!
//! * **SRT** (SubRip) — `HH:MM:SS,mmm --> HH:MM:SS,mmm` timestamps,
//!   blank-line separated cues, optional numeric index line.
//! * **WebVTT** (`.vtt`) — `HH:MM:SS.mmm --> HH:MM:SS.mmm` timestamps,
//!   mandatory `WEBVTT` header, optional `NOTE` / `STYLE` / region
//!   blocks which are ignored by this implementation.
//!
//! Parser design goals:
//!
//! * Tolerant — malformed cues are skipped, not fatal, so a single bad
//!   entry does not reject the whole file.
//! * Allocation-light — timestamps are parsed into `u32` milliseconds
//!   straight away; cue text is trimmed and joined with `\n`.
//! * Pure Rust — no `web-sys` dependencies so the whole module is
//!   covered by native unit tests.

use core::fmt;

use message::types::SubtitleEntry;

/// Error returned by [`parse_srt`] / [`parse_vtt`] when the whole file
/// cannot be parsed at all (wrong header, empty input, …).
///
/// Cue-level recovery is handled internally — only structural errors
/// propagate as [`SubtitleParseError`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubtitleParseError {
  /// Input was empty or contained only whitespace.
  Empty,
  /// WebVTT header (`WEBVTT`) was missing or misspelled.
  MissingWebVttHeader,
  /// Neither SRT nor WebVTT markers were detected.
  UnknownFormat,
}

impl fmt::Display for SubtitleParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Empty => write!(f, "Subtitle file is empty"),
      Self::MissingWebVttHeader => write!(f, "WebVTT file is missing the 'WEBVTT' header"),
      Self::UnknownFormat => write!(f, "Subtitle format not recognised (expected SRT or WebVTT)"),
    }
  }
}

impl std::error::Error for SubtitleParseError {}

/// Detect subtitle format by filename extension and dispatch to the
/// matching parser. When the extension is absent or unknown the input
/// is sniffed for a `WEBVTT` header first, falling back to SRT.
pub fn parse_subtitle_file(
  filename: &str,
  content: &str,
) -> Result<Vec<SubtitleEntry>, SubtitleParseError> {
  let lower = filename.to_ascii_lowercase();
  if lower.ends_with(".vtt") {
    return parse_vtt(content);
  }
  if lower.ends_with(".srt") {
    return parse_srt(content);
  }
  // Fall back to sniffing: WEBVTT header wins, otherwise try SRT.
  let trimmed = content.trim_start_matches('\u{feff}').trim_start();
  if trimmed
    .lines()
    .next()
    .is_some_and(|line| line.trim_start().starts_with("WEBVTT"))
  {
    parse_vtt(content)
  } else if trimmed.is_empty() {
    Err(SubtitleParseError::Empty)
  } else {
    parse_srt(content)
  }
}

/// Parse SRT (SubRip) subtitle content into a sorted entry list.
///
/// The parser is lenient — malformed cues are skipped silently so a
/// stray blank line or missing index does not reject the whole file.
///
/// # Errors
/// Returns [`SubtitleParseError::Empty`] when the input contains no
/// printable characters.
pub fn parse_srt(content: &str) -> Result<Vec<SubtitleEntry>, SubtitleParseError> {
  let trimmed = content.trim_start_matches('\u{feff}').trim();
  if trimmed.is_empty() {
    return Err(SubtitleParseError::Empty);
  }

  let mut entries = Vec::new();

  // SRT cues are separated by blank lines. A cue is typically:
  //   1\n
  //   00:00:10,500 --> 00:00:13,000\n
  //   Hello world\n
  //   Second line\n
  //
  // Some authoring tools drop the leading index; we handle both.
  for block in split_blank_line_blocks(trimmed) {
    let mut lines = block.lines().map(str::trim).filter(|l| !l.is_empty());
    let Some(mut first) = lines.next() else {
      continue;
    };

    // Optional numeric index line — if first line is purely digits
    // and next line contains " --> ", consume the index.
    if first.chars().all(|c| c.is_ascii_digit())
      && let Some(next) = lines.next()
    {
      first = next;
    }

    let Some((start, end)) = parse_timestamp_line(first, ',') else {
      continue;
    };

    let text: String = lines.collect::<Vec<_>>().join("\n");
    if text.is_empty() {
      continue;
    }

    entries.push(SubtitleEntry {
      start_ms: start,
      end_ms: end,
      text,
    });
  }

  entries.sort_by_key(|e| e.start_ms);
  Ok(entries)
}

/// Parse WebVTT (.vtt) subtitle content into a sorted entry list.
///
/// `NOTE`, `STYLE`, and `REGION` blocks are ignored.
///
/// # Errors
/// Returns [`SubtitleParseError::MissingWebVttHeader`] when the first
/// non-empty line is not `WEBVTT` (optionally followed by metadata),
/// or [`SubtitleParseError::Empty`] when the input is blank.
pub fn parse_vtt(content: &str) -> Result<Vec<SubtitleEntry>, SubtitleParseError> {
  let trimmed = content.trim_start_matches('\u{feff}').trim_start();
  if trimmed.is_empty() {
    return Err(SubtitleParseError::Empty);
  }

  let mut lines_iter = trimmed.lines();
  let header = lines_iter.next().unwrap_or("").trim();
  if !header.starts_with("WEBVTT") {
    return Err(SubtitleParseError::MissingWebVttHeader);
  }

  // Rebuild the remaining body for block splitting.
  let body: String = lines_iter.collect::<Vec<_>>().join("\n");
  if body.trim().is_empty() {
    return Ok(Vec::new());
  }

  let mut entries = Vec::new();
  for block in split_blank_line_blocks(&body) {
    let block = block.trim();
    if block.is_empty() {
      continue;
    }
    // Skip meta blocks.
    let first_word = block.split_whitespace().next().unwrap_or("");
    if matches!(first_word, "NOTE" | "STYLE" | "REGION") {
      continue;
    }

    let mut lines = block.lines().map(str::trim).filter(|l| !l.is_empty());
    let Some(mut header_line) = lines.next() else {
      continue;
    };

    // WebVTT allows an optional cue identifier line before the
    // timestamp — skip it when the timestamp isn't present yet.
    if !header_line.contains("-->")
      && let Some(next) = lines.next()
    {
      header_line = next;
    }

    let Some((start, end)) = parse_timestamp_line(header_line, '.') else {
      continue;
    };

    let text: String = lines.collect::<Vec<_>>().join("\n");
    if text.is_empty() {
      continue;
    }

    entries.push(SubtitleEntry {
      start_ms: start,
      end_ms: end,
      text,
    });
  }

  entries.sort_by_key(|e| e.start_ms);
  Ok(entries)
}

/// Split a string on blank-line boundaries, returning non-empty blocks.
///
/// Handles LF and CRLF line endings; empty lines only made of
/// whitespace are treated as separators. Normalises line endings once
/// upfront, then splits on double-newlines to avoid per-line String
/// allocations.
fn split_blank_line_blocks(input: &str) -> Vec<String> {
  // Normalise all line endings to LF in a single pass.
  let normalised = input.replace("\r\n", "\n").replace('\r', "\n");
  // Collapse runs of blank lines (lines with only whitespace) into a
  // single "\n\n" separator so `split` works reliably.
  let mut result = Vec::new();
  let mut current = String::new();
  for line in normalised.split('\n') {
    if line.trim().is_empty() {
      if !current.is_empty() {
        result.push(std::mem::take(&mut current));
      }
    } else {
      if !current.is_empty() {
        current.push('\n');
      }
      current.push_str(line);
    }
  }
  if !current.is_empty() {
    result.push(current);
  }
  result
}

/// Parse a `HH:MM:SS{separator}mmm --> HH:MM:SS{separator}mmm` line.
///
/// `fractional_separator` is `,` for SRT and `.` for WebVTT.
fn parse_timestamp_line(line: &str, fractional_separator: char) -> Option<(u32, u32)> {
  // WebVTT cues can have trailing settings after the end timestamp,
  // e.g. `00:00:10.000 --> 00:00:13.000 line:80% align:center`.
  let (times, _settings) = line
    .split_once(' ')
    .map_or((line, ""), |(t, s)| (t.trim(), s));
  // After the split we still need to locate `-->` inside `times` plus
  // any remaining content, so re-assemble and split on the arrow.
  let whole = line.trim();
  let (start_raw, end_raw) = whole.split_once("-->")?;
  let start_trimmed = start_raw.trim();
  let end_trimmed_with_settings = end_raw.trim();
  // Drop trailing settings if present.
  let end_trimmed = end_trimmed_with_settings
    .split_whitespace()
    .next()
    .unwrap_or(end_trimmed_with_settings);
  let start_ms = parse_timestamp(start_trimmed, fractional_separator)?;
  let end_ms = parse_timestamp(end_trimmed, fractional_separator)?;
  if end_ms <= start_ms {
    return None;
  }
  // `times` is intentionally unused once we've parsed the timestamps —
  // mark it as consumed to silence a future dead-code warning.
  let _ = times;
  Some((start_ms, end_ms))
}

/// Parse a timestamp like `HH:MM:SS,mmm` or `MM:SS.mmm` into milliseconds.
fn parse_timestamp(input: &str, fractional_separator: char) -> Option<u32> {
  let (time_part, frac_part) = input.split_once(fractional_separator)?;
  let frac_ms: u32 = frac_part.trim().parse().ok()?;
  let components: Vec<&str> = time_part.split(':').collect();
  let (hours, minutes, seconds) = match components.as_slice() {
    [h, m, s] => (
      h.parse::<u32>().ok()?,
      m.parse::<u32>().ok()?,
      s.parse::<u32>().ok()?,
    ),
    [m, s] => (0, m.parse::<u32>().ok()?, s.parse::<u32>().ok()?),
    _ => return None,
  };
  if minutes >= 60 || seconds >= 60 || frac_ms >= 1000 {
    return None;
  }
  Some(((hours * 3_600) + (minutes * 60) + seconds) * 1_000 + frac_ms)
}

/// Pick the subtitle entry that should be displayed at the given
/// playback timestamp, if any.
#[must_use]
pub fn active_entry(entries: &[SubtitleEntry], time_ms: u32) -> Option<&SubtitleEntry> {
  // Binary search by start_ms, then walk back while the timestamp
  // still fits the previous cue — handles overlapping cues.
  let idx = entries.partition_point(|e| e.start_ms <= time_ms);
  entries[..idx]
    .iter()
    .rev()
    .find(|e| e.is_active_at(time_ms))
}

#[cfg(test)]
mod tests;
