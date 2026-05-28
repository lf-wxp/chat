//! Per-peer network quality indicator (4-bar signal icon).

use leptos::prelude::*;
use leptos_i18n::t_string;
use message::UserId;
use message::types::NetworkQuality;

use crate::call::{ConnectionType, NetworkStatsSample, use_call_signals};
use crate::i18n;
use crate::state::use_app_state;

/// Whether the n-th bar (1-indexed) of the 4-bar signal icon should
/// render in the "active" state for the given [`NetworkQuality`].
///
/// Excellent → 4 bars, Good → 3, Fair → 2, Poor → 1.
///
/// Exposed as a pure function so the mapping can be exercised by unit
/// tests without mounting the full [`NetworkIndicator`] component
#[must_use]
pub fn bar_is_active(quality: NetworkQuality, bar: usize) -> bool {
  match quality {
    NetworkQuality::Excellent => bar <= 4,
    NetworkQuality::Good => bar <= 3,
    NetworkQuality::Fair => bar <= 2,
    NetworkQuality::Poor => bar <= 1,
  }
}

/// Whether a bar should render active given an *optional* quality
/// when no `getStats()` sample has been collected
/// yet (e.g. during the first 5 s after a peer connects, or when the
/// browser returns an empty stats report), the indicator must NOT
/// pretend the network is "Good"; instead every bar stays inactive
/// (rendered grey) until real data arrives.
#[must_use]
pub fn bar_is_active_opt(quality: Option<NetworkQuality>, bar: usize) -> bool {
  match quality {
    Some(q) => bar_is_active(q, bar),
    None => false,
  }
}

/// Build the `title` tooltip text for the network indicator from a
/// localised quality label and an optional detail suffix.
///
/// The detail suffix is typically " · RTT: Xms · Loss: Y.Y%" when a
/// stats sample is available, or empty otherwise. Exposed as a pure
/// function so the formatting is unit-testable.
#[must_use]
pub fn format_tooltip(quality_label: &str, detail: &str) -> String {
  format!("{quality_label}{detail}")
}

/// Format the millisecond bandwidth value (kbps) into a human-friendly
/// "X.Y Mbps" or "N kbps" string. Helper for [`format_detail`].
#[must_use]
pub fn format_bandwidth(kbps: u32) -> String {
  if kbps >= 1_000 {
    let mbps = f64::from(kbps) / 1_000.0;
    format!("{mbps:.1} Mbps")
  } else {
    format!("{kbps} kbps")
  }
}

/// Build the detail suffix appended after the localised quality label
/// in the network-indicator tooltip (Req 14.10.3).
///
/// Exposed as a pure function so the formatting can be exercised by
/// unit / wasm tests without mounting the full component.
///
/// Format:
/// ```text
///  · RTT: 42ms · Loss: 0.0%[ · Bandwidth: 4.2 Mbps][ · Connection: Direct (P2P)]
/// ```
///
/// `bandwidth_label` and `connection_label` are the localised strings
/// for "Bandwidth" / "Connection" (passed in so the function stays
/// pure — no i18n context dependency). `connection_type_label` is the
/// already-localised value-string ("Direct (P2P)" / "Relayed (TURN)" /
/// "Unknown"). The bandwidth and connection segments are omitted when
/// not yet known so the tooltip degrades gracefully on first samples.
#[must_use]
pub fn format_detail(
  sample: Option<&NetworkStatsSample>,
  bandwidth_label: &str,
  connection_label: &str,
  connection_type_label: &str,
) -> String {
  let Some(s) = sample else {
    return String::new();
  };
  let mut detail = format!(" · RTT: {}ms · Loss: {:.1}%", s.rtt_ms, s.loss_percent);
  if let Some(kbps) = s.bandwidth_kbps {
    detail.push_str(&format!(" · {bandwidth_label}: {}", format_bandwidth(kbps)));
  }
  if !matches!(s.connection_type, ConnectionType::Unknown) {
    detail.push_str(&format!(" · {connection_label}: {connection_type_label}"));
  }
  detail
}

/// Map a [`NetworkQuality`] to the kebab-case slug rendered in the
/// `data-quality` attribute. The CSS layer keys the Poor pulse
/// animation off this attribute (Req 14.10.4).
///
/// Returns `"unknown"` when no sample has been collected yet so the
/// stylesheet can paint a neutral grey state instead of optimistically
/// claiming the connection is healthy.
#[must_use]
pub fn quality_data_attr(quality: Option<NetworkQuality>) -> &'static str {
  match quality {
    Some(NetworkQuality::Excellent) => "excellent",
    Some(NetworkQuality::Good) => "good",
    Some(NetworkQuality::Fair) => "fair",
    Some(NetworkQuality::Poor) => "poor",
    None => "unknown",
  }
}

/// Network-quality indicator component.
///
/// Renders a 4-bar signal icon whose filled count reflects the current
/// [`NetworkQuality`] for the given peer. On hover, the `title`
/// attribute shows the quality label plus the latest RTT, packet-loss,
/// bandwidth, and connection-type figures (Req 14.10.3).
///
/// When no quality sample has been collected yet, every bar
/// stays inactive and the tooltip displays the localised "Unknown"
/// label so the user is not misled into thinking the connection is
/// healthy when the indicator is actually waiting on first data.
#[component]
pub fn NetworkIndicator(peer_id: UserId) -> impl IntoView {
  let app_state = use_app_state();
  let signals = use_call_signals();
  let i18n = i18n::use_i18n();
  let peer_for_quality = peer_id.clone();
  let peer_for_stats = peer_id.clone();

  let quality = Memo::new(move |_| {
    app_state
      .network_quality
      .with(|map| map.get(&peer_for_quality).copied())
  });

  let active_bars = move |bars: usize| bar_is_active_opt(quality.get(), bars);

  // Build a detailed tooltip with RTT, loss, bandwidth, and
  // connection-type figures (Req 14.10.3). The quality label and
  // metric labels are resolved through i18n so they match the user's
  // locale.
  let tooltip = Memo::new(move |_| {
    let quality_label = match quality.get() {
      Some(NetworkQuality::Excellent) => t_string!(i18n, call.quality_excellent),
      Some(NetworkQuality::Good) => t_string!(i18n, call.quality_good),
      Some(NetworkQuality::Fair) => t_string!(i18n, call.quality_fair),
      Some(NetworkQuality::Poor) => t_string!(i18n, call.quality_poor),
      None => t_string!(i18n, call.quality_unknown),
    };
    let bandwidth_label = t_string!(i18n, call.bandwidth_label);
    let connection_label = t_string!(i18n, call.connection_type_label);
    let detail = signals.network_stats.with(|map| {
      let sample = map.get(&peer_for_stats);
      let connection_type_label = sample
        .map(|s| match s.connection_type {
          ConnectionType::Direct => t_string!(i18n, call.connection_type_direct),
          ConnectionType::Relayed => t_string!(i18n, call.connection_type_relayed),
          ConnectionType::Unknown => t_string!(i18n, call.connection_type_unknown),
        })
        .unwrap_or_else(|| t_string!(i18n, call.connection_type_unknown));
      format_detail(
        sample,
        bandwidth_label,
        connection_label,
        connection_type_label,
      )
    });
    format_tooltip(quality_label, &detail)
  });

  view! {
    <span
      class="network-indicator"
      class:network-indicator--unknown=move || quality.get().is_none()
      role="img"
      aria-label=move || t_string!(i18n, call.network_quality)
      title=move || tooltip.get()
      attr:data-quality=move || quality_data_attr(quality.get())
    >
      <span class="network-indicator__bar" class:is-active=move || active_bars(1)></span>
      <span class="network-indicator__bar" class:is-active=move || active_bars(2)></span>
      <span class="network-indicator__bar" class:is-active=move || active_bars(3)></span>
      <span class="network-indicator__bar" class:is-active=move || active_bars(4)></span>
    </span>
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample_full() -> NetworkStatsSample {
    NetworkStatsSample {
      rtt_ms: 42,
      loss_percent: 0.0,
      bandwidth_kbps: Some(4_200),
      connection_type: ConnectionType::Direct,
      sampled_at_ms: 0,
    }
  }

  fn sample_minimal() -> NetworkStatsSample {
    NetworkStatsSample {
      rtt_ms: 250,
      loss_percent: 4.5,
      bandwidth_kbps: None,
      connection_type: ConnectionType::Unknown,
      sampled_at_ms: 0,
    }
  }

  #[test]
  fn excellent_activates_all_four_bars() {
    for bar in 1..=4 {
      assert!(
        bar_is_active(NetworkQuality::Excellent, bar),
        "bar {bar} should be active for Excellent"
      );
    }
  }

  #[test]
  fn good_activates_first_three_bars() {
    assert!(bar_is_active(NetworkQuality::Good, 1));
    assert!(bar_is_active(NetworkQuality::Good, 2));
    assert!(bar_is_active(NetworkQuality::Good, 3));
    assert!(!bar_is_active(NetworkQuality::Good, 4));
  }

  #[test]
  fn fair_activates_first_two_bars() {
    assert!(bar_is_active(NetworkQuality::Fair, 1));
    assert!(bar_is_active(NetworkQuality::Fair, 2));
    assert!(!bar_is_active(NetworkQuality::Fair, 3));
    assert!(!bar_is_active(NetworkQuality::Fair, 4));
  }

  #[test]
  fn poor_activates_only_first_bar() {
    assert!(bar_is_active(NetworkQuality::Poor, 1));
    assert!(!bar_is_active(NetworkQuality::Poor, 2));
    assert!(!bar_is_active(NetworkQuality::Poor, 3));
    assert!(!bar_is_active(NetworkQuality::Poor, 4));
  }

  #[test]
  fn tooltip_appends_detail_when_present() {
    let t = format_tooltip("Excellent", " · RTT: 42ms · Loss: 0.0%");
    assert_eq!(t, "Excellent · RTT: 42ms · Loss: 0.0%");
  }

  #[test]
  fn tooltip_is_plain_label_when_no_detail() {
    // When no stats sample is available the indicator falls back to
    // just the localised quality label, with no trailing separator.
    let t = format_tooltip("良好", "");
    assert_eq!(t, "良好");
  }

  #[test]
  fn no_quality_keeps_all_bars_inactive() {
    // until a real stats sample arrives, every bar must stay
    // inactive — rendering "Good" by default would falsely advertise
    // a healthy connection during the initial 5 s polling window.
    for bar in 1..=4 {
      assert!(
        !bar_is_active_opt(None, bar),
        "bar {bar} must stay inactive when no quality reading is available",
      );
    }
  }

  #[test]
  fn known_quality_delegates_to_bar_is_active() {
    // Sanity check: passing Some(quality) yields the same answer as
    // calling `bar_is_active` directly.
    for bar in 1..=4 {
      assert_eq!(
        bar_is_active_opt(Some(NetworkQuality::Fair), bar),
        bar_is_active(NetworkQuality::Fair, bar),
      );
    }
  }

  #[test]
  fn quality_data_attr_maps_each_variant() {
    assert_eq!(
      quality_data_attr(Some(NetworkQuality::Excellent)),
      "excellent"
    );
    assert_eq!(quality_data_attr(Some(NetworkQuality::Good)), "good");
    assert_eq!(quality_data_attr(Some(NetworkQuality::Fair)), "fair");
    assert_eq!(quality_data_attr(Some(NetworkQuality::Poor)), "poor");
  }

  #[test]
  fn quality_data_attr_unknown_for_missing_sample() {
    // The CSS pulse rule must NOT trigger before a real sample is
    // collected. Returning "unknown" keeps the indicator neutral.
    assert_eq!(quality_data_attr(None), "unknown");
  }

  // ── R1: format_detail / format_bandwidth coverage ──

  #[test]
  fn format_bandwidth_kbps_below_1_mbps() {
    assert_eq!(format_bandwidth(750), "750 kbps");
    assert_eq!(format_bandwidth(1), "1 kbps");
  }

  #[test]
  fn format_bandwidth_mbps_at_or_above_1_mbps() {
    assert_eq!(format_bandwidth(1_000), "1.0 Mbps");
    assert_eq!(format_bandwidth(4_200), "4.2 Mbps");
    assert_eq!(format_bandwidth(12_500), "12.5 Mbps");
  }

  #[test]
  fn format_detail_returns_empty_when_no_sample() {
    assert_eq!(
      format_detail(None, "Bandwidth", "Connection", "Unknown"),
      ""
    );
  }

  #[test]
  fn format_detail_contains_rtt_loss_bandwidth_and_connection_when_known() {
    let s = sample_full();
    let d = format_detail(Some(&s), "Bandwidth", "Connection", "Direct (P2P)");
    assert!(d.contains("RTT: 42ms"), "expected RTT segment in {d}");
    assert!(d.contains("Loss: 0.0%"), "expected loss segment in {d}");
    assert!(
      d.contains("Bandwidth: 4.2 Mbps"),
      "expected bandwidth segment in {d}",
    );
    assert!(
      d.contains("Connection: Direct (P2P)"),
      "expected connection-type segment in {d}",
    );
  }

  #[test]
  fn format_detail_omits_bandwidth_when_unknown() {
    let s = sample_minimal();
    let d = format_detail(Some(&s), "Bandwidth", "Connection", "Unknown");
    assert!(!d.contains("Bandwidth"));
  }

  #[test]
  fn format_detail_omits_connection_when_type_unknown() {
    let s = sample_minimal();
    let d = format_detail(Some(&s), "Bandwidth", "Connection", "Unknown");
    assert!(
      !d.contains("Connection"),
      "Unknown connection type must be hidden, got {d}",
    );
  }
}
