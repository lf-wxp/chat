//! Network Suggestions Component
//!
//! Smart suggestions based on network quality

use crate::network_quality::QualityLevel;
use crate::{i18n::*, state};
use leptos::prelude::*;
use leptos_i18n::t_string;

#[component]
pub fn NetworkSuggestions() -> impl IntoView {
  let nq_state = state::use_network_quality_state();
  let i18n = use_i18n();

  let suggestions_signal = Signal::derive(move || {
    let mut suggestions: Vec<String> = Vec::new();

    nq_state.with(|s| {
      // Check overall quality
      let worst = s.worst_quality();
      if worst == QualityLevel::Poor {
        suggestions.push(t_string!(i18n, network_suggestion_poor).to_string());
      } else if worst == QualityLevel::Fair {
        suggestions.push(t_string!(i18n, network_suggestion_fair).to_string());
      }

      // Check for high RTT
      for stats in s.peer_stats.values() {
        if stats.rtt_ms > 200.0 {
          suggestions.push(
            t_string!(i18n, network_suggestion_high_latency)
              .to_string()
              .replace("{}", &(stats.rtt_ms as u32).to_string()),
          );
          break;
        }
      }

      // Check for packet loss
      for stats in s.peer_stats.values() {
        if stats.packet_loss > 0.05 {
          suggestions.push(
            t_string!(i18n, network_suggestion_packet_loss)
              .to_string()
              .replace("{:.1}", &format!("{:.1}", stats.packet_loss * 100.0)),
          );
          break;
        }
      }

      // Check for unstable connection
      if s.unacknowledged_alert_count > 3 {
        suggestions.push(t_string!(i18n, network_suggestion_multiple_issues).to_string());
      }
    });

    suggestions
  });

  view! {
    <div class="network-suggestions">
      <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
        {t_string!(i18n, network_suggestions)}
      </h3>
      <div class="space-y-2">
        <For each={move || suggestions_signal.get()} key={|s: &String| s.clone()} let:suggestion>
          {
            view! {
              <div class="flex items-start gap-2 p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
                <span class="text-blue-500">"💡"</span>
                <p class="text-sm text-blue-900 dark:text-blue-100">{suggestion}</p>
              </div>
            }
          }
        </For>

        <Show when={move || suggestions_signal.get().is_empty()}>
          <div class="text-center py-2 text-gray-500 dark:text-gray-400 text-sm">
          {t_string!(i18n, network_no_suggestions)}
          </div>
        </Show>
      </div>
    </div>
  }
}
