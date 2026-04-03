//! Peer Statistics Components
//!
//! Components for displaying peer network statistics

use crate::network_quality::PeerNetworkHistory;
use crate::{i18n::*, state};
use leptos::prelude::*;
use leptos_i18n::t_string;

use super::history_chart::{HistoryChart, QualityDistribution};
use super::metric_card::StatItem;
use super::quality_indicator::QualityBadge;

#[component]
pub fn PeerStatsList() -> impl IntoView {
  let nq_state = state::use_network_quality_state();
  let i18n = use_i18n();

  let peers_signal = Signal::derive(move || {
    nq_state.with(|s| {
      s.peer_stats
        .iter()
        .map(|(id, stats)| (id.clone(), stats.clone()))
        .collect::<Vec<_>>()
    })
  });

  view! {
    <div class="peer-stats-list space-y-3">
      <For each={move || peers_signal.get()} key={|(id, _)| id.clone()} let:item>
        {
          let peer_id = item.0.clone();
          let stats = item.1.clone();
          let peer_id_for_history = peer_id.clone();
          let history_signal = Signal::derive(move || {
            nq_state.with(|s| s.peer_history.get(&peer_id_for_history).cloned())
          });

          view! {
            <PeerStatsCard
              peer_id={peer_id}
              stats={stats}
              history={history_signal}
            />
          }
        }
      </For>

      <Show when={move || peers_signal.get().is_empty()}>
        <div class="text-center py-8 text-gray-500 dark:text-gray-400">
          <p class="text-4xl mb-2">"📡"</p>
          <p class="text-sm">{t_string!(i18n, network_no_connections)}</p>
        </div>
      </Show>
    </div>
  }
}

#[component]
fn PeerStatsCard(
  peer_id: String,
  stats: crate::network_quality::NetworkStats,
  history: Signal<Option<PeerNetworkHistory>>,
) -> impl IntoView {
  let i18n = use_i18n();
  let is_expanded = RwSignal::new(false);

  view! {
    <div class="peer-stats-card bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
      // Header - always visible
      <button
        class="w-full p-4 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
        on:click={move |_| is_expanded.update(|v| *v = !*v)}
      >
        <div class="flex items-center gap-3">
          <QualityBadge quality={stats.quality} />
          <span class="text-sm font-medium text-gray-900 dark:text-white truncate max-w-[200px]">
            {peer_id.clone()}
          </span>
        </div>

        <div class="flex items-center gap-4 text-xs text-gray-500 dark:text-gray-400">
          <span title=t_string!(i18n, network_rtt)>"⏱️ " {format!("{:.0}ms", stats.rtt_ms)}</span>
          <span title=t_string!(i18n, network_packet_loss)>"📦 " {format!("{:.1}%", stats.packet_loss * 100.0)}</span>
          <span title=t_string!(i18n, network_bitrate)>"📊 " {format!("{:.0}kbps", stats.current_bitrate / 1000.0)}</span>
          <span class="transform transition-transform" class:rotate-180={is_expanded}>
            "▼"
          </span>
        </div>
      </button>

      // Expanded details
      <Show when={move || is_expanded.get()}>
        <div class="border-t border-gray-200 dark:border-gray-700 p-4">
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
            <StatItem label=t_string!(i18n, network_rtt).to_string() value={format!("{:.1} ms", stats.rtt_ms)} icon="⏱️" />
            <StatItem label=t_string!(i18n, network_packet_loss).to_string() value={format!("{:.2}%", stats.packet_loss * 100.0)} icon="📦" />
            <StatItem label=t_string!(i18n, network_jitter).to_string() value={format!("{:.1} ms", stats.jitter_ms)} icon="📈" />
            <StatItem label=t_string!(i18n, network_bitrate).to_string() value={format!("{:.0} kbps", stats.current_bitrate / 1000.0)} icon="📊" />
          </div>

          // History chart
          <div class="mt-4">
            <h4 class="text-xs font-medium text-gray-500 dark:text-gray-400 mb-2">
              {t_string!(i18n, network_history_title)}
            </h4>
            <HistoryChart history={history} metric="rtt" />
          </div>

          // Quality distribution
          <Show when={move || history.get().is_some()}>
            <QualityDistribution history={history} />
          </Show>
        </div>
      </Show>
    </div>
  }
}
