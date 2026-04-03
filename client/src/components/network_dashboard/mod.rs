//! Network Quality Dashboard Component
//!
//! Provides comprehensive network quality monitoring UI including:
//! - Real-time statistics display
//! - Historical data charts
//! - Alert notifications
//! - Quality indicators

use crate::{i18n::*, state};
use leptos::prelude::*;
use leptos_i18n::t_string;

// Sub-modules
mod alerts;
mod history_chart;
mod metric_card;
mod peer_stats;
mod quality_indicator;
mod suggestions;

// Re-exports
pub use alerts::NetworkAlertsPanel;
pub use metric_card::MetricCard;
pub use peer_stats::PeerStatsList;
pub use quality_indicator::{QualityBadge, QualityIndicator};

// =============================================================================
// Network Dashboard - Full View
// =============================================================================

/// Full network quality dashboard component
#[component]
pub fn NetworkDashboard(
  /// Whether to show the dashboard
  #[prop(default = true)]
  show: bool,
) -> impl IntoView {
  let nq_state = state::use_network_quality_state();

  let i18n = use_i18n();

  // Get overall statistics
  let (peer_count, worst_quality, unacknowledged_count) = nq_state.with_untracked(|s| {
    (
      s.peer_stats.len(),
      s.worst_quality(),
      s.unacknowledged_alert_count,
    )
  });

  view! {
    <div class="network-dashboard" class:hidden={!show}>
      // Header with overall status
      <div class="dashboard-header">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-lg font-semibold text-gray-900 dark:text-white">
            {t_string!(i18n, network_title)}
          </h2>
          <QualityBadge quality={worst_quality} />
        </div>

        // Summary metrics
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
          <MetricCard
            label=t_string!(i18n, network_active_peers).to_string()
            value={peer_count.to_string()}
            icon="👥"
          />
          <MetricCard
            label=t_string!(i18n, network_quality).to_string()
            value={worst_quality.label().to_string()}
            icon={worst_quality.icon()}
            color={worst_quality.color()}
          />
          <MetricCard
            label=t_string!(i18n, network_alerts).to_string()
            value={unacknowledged_count.to_string()}
            icon="⚠️"
            color={if unacknowledged_count > 0 { "#ef4444" } else { "#22c55e" }}
          />
          <MetricCard
            label=t_string!(i18n, network_status).to_string()
            value={if peer_count == 0 { t_string!(i18n, network_idle).to_string() } else { t_string!(i18n, network_active).to_string() }}
            icon="📡"
          />
        </div>
      </div>

      // Peer details section
      <div class="dashboard-content mt-6">
        <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
          {t_string!(i18n, network_peer_details)}
        </h3>
        <PeerStatsList />
      </div>

      // Alerts section
      <div class="dashboard-alerts mt-6">
        <NetworkAlertsPanel show_acknowledged={false} />
      </div>
    </div>
  }
}

// =============================================================================
// Compact Dashboard View
// =============================================================================

/// Compact network quality indicator for header/toolbar
#[component]
pub fn NetworkDashboardCompact() -> impl IntoView {
  let nq_state = state::use_network_quality_state();

  let quality_signal = Signal::derive(move || nq_state.with(|s| s.worst_quality()));

  let alert_count_signal = Signal::derive(move || nq_state.with(|s| s.unacknowledged_alert_count));

  view! {
    <div class="network-dashboard-compact flex items-center gap-2">
      <QualityIndicator quality={quality_signal} />

      <Show when={move || alert_count_signal.get() > 0}>
        <span class="flex items-center justify-center w-5 h-5 text-xs font-bold text-white bg-red-500 rounded-full">
          {alert_count_signal}
        </span>
      </Show>
    </div>
  }
}
