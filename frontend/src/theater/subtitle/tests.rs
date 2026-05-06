//! Unit tests for SRT / WebVTT subtitle parsers.

use super::*;

#[test]
fn parse_srt_handles_basic_cues() {
  let srt = "1\n00:00:01,000 --> 00:00:03,500\nHello world\n\n\
             2\n00:00:05,200 --> 00:00:06,800\nSecond line\n";
  let entries = parse_srt(srt).expect("valid SRT");
  assert_eq!(entries.len(), 2);
  assert_eq!(entries[0].start_ms, 1_000);
  assert_eq!(entries[0].end_ms, 3_500);
  assert_eq!(entries[0].text, "Hello world");
  assert_eq!(entries[1].start_ms, 5_200);
  assert_eq!(entries[1].end_ms, 6_800);
}

#[test]
fn parse_srt_supports_multiline_cues() {
  let srt = "1\n00:00:10,000 --> 00:00:12,000\nLine one\nLine two\n";
  let entries = parse_srt(srt).expect("valid SRT");
  assert_eq!(entries.len(), 1);
  assert_eq!(entries[0].text, "Line one\nLine two");
}

#[test]
fn parse_srt_skips_malformed_blocks_and_keeps_good_ones() {
  let srt = "1\nBAD TIMESTAMP\nGarbage\n\n\
             2\n00:00:04,000 --> 00:00:05,000\nOK\n";
  let entries = parse_srt(srt).expect("valid SRT");
  assert_eq!(entries.len(), 1);
  assert_eq!(entries[0].text, "OK");
}

#[test]
fn parse_srt_sorts_by_start_ms() {
  let srt = "1\n00:00:05,000 --> 00:00:06,000\nSecond\n\n\
             2\n00:00:01,000 --> 00:00:02,000\nFirst\n";
  let entries = parse_srt(srt).expect("valid SRT");
  assert_eq!(entries[0].text, "First");
  assert_eq!(entries[1].text, "Second");
}

#[test]
fn parse_srt_rejects_empty_input() {
  assert_eq!(parse_srt(""), Err(SubtitleParseError::Empty));
  assert_eq!(parse_srt("   \n\n  "), Err(SubtitleParseError::Empty));
}

#[test]
fn parse_srt_handles_crlf_line_endings() {
  let srt = "1\r\n00:00:01,000 --> 00:00:02,000\r\nHello\r\n\r\n";
  let entries = parse_srt(srt).expect("valid SRT");
  assert_eq!(entries.len(), 1);
  assert_eq!(entries[0].text, "Hello");
}

#[test]
fn parse_vtt_handles_basic_cues() {
  let vtt = "WEBVTT\n\n\
             00:00:01.000 --> 00:00:03.500\nHello world\n\n\
             00:00:05.200 --> 00:00:06.800\nSecond line\n";
  let entries = parse_vtt(vtt).expect("valid VTT");
  assert_eq!(entries.len(), 2);
  assert_eq!(entries[0].start_ms, 1_000);
  assert_eq!(entries[0].end_ms, 3_500);
  assert_eq!(entries[1].start_ms, 5_200);
}

#[test]
fn parse_vtt_rejects_missing_header() {
  let vtt = "00:00:01.000 --> 00:00:02.000\nMissing header\n";
  assert_eq!(parse_vtt(vtt), Err(SubtitleParseError::MissingWebVttHeader));
}

#[test]
fn parse_vtt_ignores_note_and_style_blocks() {
  let vtt = "WEBVTT\n\n\
             NOTE This is a comment block\nignored\n\n\
             STYLE\n::cue { color: red }\n\n\
             00:00:02.000 --> 00:00:03.000\nVisible\n";
  let entries = parse_vtt(vtt).expect("valid VTT");
  assert_eq!(entries.len(), 1);
  assert_eq!(entries[0].text, "Visible");
}

#[test]
fn parse_vtt_supports_cue_identifier_line() {
  let vtt = "WEBVTT\n\n\
             cue-id-1\n00:00:04.000 --> 00:00:05.000\nWith ID\n";
  let entries = parse_vtt(vtt).expect("valid VTT");
  assert_eq!(entries.len(), 1);
  assert_eq!(entries[0].text, "With ID");
}

#[test]
fn parse_vtt_tolerates_trailing_settings() {
  let vtt = "WEBVTT\n\n\
             00:00:06.000 --> 00:00:07.000 line:80% align:center\nTrailing\n";
  let entries = parse_vtt(vtt).expect("valid VTT");
  assert_eq!(entries.len(), 1);
  assert_eq!(entries[0].text, "Trailing");
}

#[test]
fn parse_subtitle_file_dispatches_by_extension() {
  let srt = "1\n00:00:01,000 --> 00:00:02,000\nSrt\n";
  let vtt = "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nVtt\n";
  assert_eq!(
    parse_subtitle_file("movie.srt", srt).expect("srt ok")[0].text,
    "Srt"
  );
  assert_eq!(
    parse_subtitle_file("movie.vtt", vtt).expect("vtt ok")[0].text,
    "Vtt"
  );
}

#[test]
fn parse_subtitle_file_sniffs_when_extension_unknown() {
  let vtt_no_ext = "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nSniffed\n";
  let entries = parse_subtitle_file("caption.txt", vtt_no_ext).expect("sniffed");
  assert_eq!(entries[0].text, "Sniffed");
}

#[test]
fn active_entry_returns_matching_cue() {
  let entries = vec![
    SubtitleEntry::new(0, 1_000, "A".into()),
    SubtitleEntry::new(1_000, 2_000, "B".into()),
    SubtitleEntry::new(2_500, 3_500, "C".into()),
  ];
  assert_eq!(active_entry(&entries, 500).unwrap().text, "A");
  assert_eq!(active_entry(&entries, 1_500).unwrap().text, "B");
  assert_eq!(active_entry(&entries, 2_000), None); // gap between cues
  assert_eq!(active_entry(&entries, 3_499).unwrap().text, "C");
  assert_eq!(active_entry(&entries, 3_500), None);
}

#[test]
fn parse_srt_handles_utf8_bom() {
  let srt = "\u{feff}1\n00:00:01,000 --> 00:00:02,000\nWith BOM\n";
  let entries = parse_srt(srt).expect("valid SRT");
  assert_eq!(entries.len(), 1);
  assert_eq!(entries[0].text, "With BOM");
}

#[test]
fn parse_srt_rejects_end_before_start() {
  let srt = "1\n00:00:05,000 --> 00:00:04,000\nBad\n";
  let entries = parse_srt(srt).expect("parses");
  assert!(entries.is_empty());
}
