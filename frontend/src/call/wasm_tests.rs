//! WASM-bindgen tests for the call subsystem.
//!
//! Covers JS-surface behaviour that cannot be exercised by the
//! native unit tests: the `getStats()` report parser, which walks a
//! live JS `Object` / `Map` of `RTCStats` entries.

use js_sys::{Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

use super::stats::parse_stats_report;

wasm_bindgen_test_configure!(run_in_browser);

/// Build an `RTCStats`-shaped dictionary as a plain JS object for the
/// test harness. Real browsers return a `Map`, but our parser relies on
/// `Object::entries` which works on both — so a plain object is the
/// simpler fixture and sufficient coverage.
fn stat_entry(entries: &[(&str, JsValue)]) -> JsValue {
  let obj = Object::new();
  for (key, value) in entries {
    Reflect::set(&obj, &JsValue::from_str(key), value).expect("set");
  }
  obj.into()
}

/// Assemble a full report object keyed by stat id → stat dict.
fn build_report(items: &[(&str, JsValue)]) -> JsValue {
  let obj = Object::new();
  for (id, value) in items {
    Reflect::set(&obj, &JsValue::from_str(id), value).expect("set");
  }
  obj.into()
}

#[wasm_bindgen_test]
fn parse_extracts_rtt_from_nominated_candidate_pair() {
  let pair = stat_entry(&[
    ("type", JsValue::from_str("candidate-pair")),
    ("nominated", JsValue::from_bool(true)),
    // 80 ms round-trip.
    ("currentRoundTripTime", JsValue::from_f64(0.080)),
  ]);
  let report = build_report(&[("cp1", pair)]);

  let sample = parse_stats_report(&report, 0).expect("nominated pair must yield a sample");
  assert_eq!(sample.rtt_ms, 80);
  assert!((sample.loss_percent - 0.0).abs() < f64::EPSILON);
}

#[wasm_bindgen_test]
fn parse_ignores_non_nominated_candidate_pair() {
  let pair = stat_entry(&[
    ("type", JsValue::from_str("candidate-pair")),
    ("nominated", JsValue::from_bool(false)),
    ("currentRoundTripTime", JsValue::from_f64(0.5)),
  ]);
  let report = build_report(&[("cp1", pair)]);

  // Non-nominated pair contributes nothing; with no other recognised
  // entries the report is treated as "no data" (H4 fix).
  assert!(parse_stats_report(&report, 0).is_none());
}

#[wasm_bindgen_test]
fn parse_computes_loss_from_inbound_rtp() {
  let rtp = stat_entry(&[
    ("type", JsValue::from_str("inbound-rtp")),
    ("packetsLost", JsValue::from_f64(10.0)),
    ("packetsReceived", JsValue::from_f64(90.0)),
  ]);
  let report = build_report(&[("rtp1", rtp)]);

  let sample = parse_stats_report(&report, 0).expect("inbound-rtp must yield a sample");
  assert!((sample.loss_percent - 10.0).abs() < 0.001);
}

#[wasm_bindgen_test]
fn parse_aggregates_multiple_inbound_streams() {
  let rtp_a = stat_entry(&[
    ("type", JsValue::from_str("inbound-rtp")),
    ("packetsLost", JsValue::from_f64(5.0)),
    ("packetsReceived", JsValue::from_f64(95.0)),
  ]);
  let rtp_v = stat_entry(&[
    ("type", JsValue::from_str("inbound-rtp")),
    ("packetsLost", JsValue::from_f64(15.0)),
    ("packetsReceived", JsValue::from_f64(85.0)),
  ]);
  let report = build_report(&[("a", rtp_a), ("v", rtp_v)]);

  let sample = parse_stats_report(&report, 0).expect("aggregated rtp must yield a sample");
  // 20 lost / 200 total = 10 %
  assert!((sample.loss_percent - 10.0).abs() < 0.001);
}

#[wasm_bindgen_test]
fn parse_handles_empty_report() {
  // H4 fix: an empty report must NOT silently classify as Excellent.
  // Returning None lets the caller skip the update so the UI renders
  // "Unknown" instead of a misleading green bar.
  let report: JsValue = Object::new().into();
  assert!(parse_stats_report(&report, 42).is_none());
}

#[wasm_bindgen_test]
fn parse_skips_unknown_stat_types() {
  let other = stat_entry(&[
    ("type", JsValue::from_str("transport")),
    ("bytesSent", JsValue::from_f64(12345.0)),
  ]);
  let report = build_report(&[("t", other)]);

  // Reports containing only unknown stat types collapse to None for
  // the same reason as the empty-report case above.
  assert!(parse_stats_report(&report, 0).is_none());
}

#[wasm_bindgen_test]
fn parse_returns_sample_when_only_inbound_rtp_is_present() {
  // Even without a candidate-pair entry (e.g. very early stats sweep)
  // a single inbound-rtp datum is enough to build a real sample.
  let rtp = stat_entry(&[
    ("type", JsValue::from_str("inbound-rtp")),
    ("packetsLost", JsValue::from_f64(0.0)),
    ("packetsReceived", JsValue::from_f64(100.0)),
  ]);
  let report = build_report(&[("only-rtp", rtp)]);

  let sample = parse_stats_report(&report, 7).expect("inbound-rtp only must still yield a sample");
  assert_eq!(sample.rtt_ms, 0);
  assert!((sample.loss_percent - 0.0).abs() < f64::EPSILON);
  assert_eq!(sample.sampled_at_ms, 7);
}
