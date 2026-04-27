//! Per-peer network quality indicator (4-bar signal icon).

use leptos::prelude::*;
use leptos_i18n::t_string;
use message::UserId;
use message::types::NetworkQuality;

use crate::call::use_call_signals;
use crate::i18n;
use crate::state::use_app_state;

/// Whether the n-th bar (1-indexed) of the 4-bar signal icon should
/// render in the "active" state for the given [`NetworkQuality`].
///
/// Excellent → 4 bars, Good → 3, Fair → 2, Poor → 1.
///
/// Exposed as a pure function so the mapping can be exercised by unit
/// tests without mounting the full [`NetworkIndicator`] component
/// (round-4 test-coverage fix).
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
/// reading. M1 fix — when no `getStats()` sample has been collected
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

/// Network-quality indicator component.
///
/// Renders a 4-bar signal icon whose filled count reflects the current
/// [`NetworkQuality`] for the given peer. On hover, the `title`
/// attribute shows the quality label plus the latest RTT / packet-loss
/// figures (UX-2 fix, Req 14.10).
///
/// When no quality sample has been collected yet (M1 fix), every bar
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

  // Build a detailed tooltip with RTT and loss figures (UX-2 fix).
  // The quality label is resolved through i18n so it matches the
  // user's locale instead of the raw English Display string.
  let tooltip = Memo::new(move |_| {
    let quality_label = match quality.get() {
      Some(NetworkQuality::Excellent) => t_string!(i18n, call.quality_excellent),
      Some(NetworkQuality::Good) => t_string!(i18n, call.quality_good),
      Some(NetworkQuality::Fair) => t_string!(i18n, call.quality_fair),
      Some(NetworkQuality::Poor) => t_string!(i18n, call.quality_poor),
      None => t_string!(i18n, call.quality_unknown),
    };
    let detail = signals.network_stats.with(|map| {
      map
        .get(&peer_for_stats)
        .map(|s| format!(" · RTT: {}ms · Loss: {:.1}%", s.rtt_ms, s.loss_percent))
        .unwrap_or_default()
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
    // M1 fix: until a real stats sample arrives, every bar must stay
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
}
