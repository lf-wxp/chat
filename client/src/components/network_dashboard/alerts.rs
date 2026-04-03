//! Network Alerts Components
//!
//! Components for displaying network alerts

use crate::network_quality::{AlertSeverity, AlertType, NetworkAlert};
use crate::{i18n::*, state};
use leptos::ev;
use leptos::prelude::*;
use leptos_i18n::t_string;

#[component]
pub fn NetworkAlertsPanel(#[prop(default = false)] show_acknowledged: bool) -> impl IntoView {
  let nq_state = state::use_network_quality_state();
  let i18n = use_i18n();

  let alerts_signal = Signal::derive(move || {
    nq_state.with(|s| {
      s.alerts
        .iter()
        .filter(|a| show_acknowledged || !a.acknowledged)
        .cloned()
        .collect::<Vec<_>>()
    })
  });

  let handle_acknowledge_all = move |_| {
    nq_state.update(|s| s.acknowledge_all_alerts());
  };

  view! {
    <div class="network-alerts-panel">
      <div class="flex items-center justify-between mb-3">
        <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">
          {t_string!(i18n, network_alerts_title)}
        </h3>
        <Show when={move || !alerts_signal.get().is_empty()}>
          <button
            class="text-xs text-blue-600 hover:text-blue-700 dark:text-blue-400"
            on:click={handle_acknowledge_all}
          >
            {t_string!(i18n, network_acknowledge_all)}
          </button>
        </Show>
      </div>

      <div class="alerts-list space-y-2 max-h-64 overflow-y-auto">
        <For each={move || alerts_signal.get()} key={|a: &NetworkAlert| a.id.clone()} let:alert>
          {
            let alert_id = alert.id.clone();
            let handle_acknowledge = Callback::new(move |_: ev::MouseEvent| {
              let id = alert_id.clone();
              nq_state.update(|s| s.acknowledge_alert(&id));
            });

            view! {
              <AlertItem alert={alert} on_acknowledge={handle_acknowledge} />
            }
          }
        </For>

        <Show when={move || alerts_signal.get().is_empty()}>
          <div class="text-center py-4 text-gray-500 dark:text-gray-400">
            <p class="text-2xl mb-1">"✅"</p>
            <p class="text-xs">{t_string!(i18n, network_no_alerts)}</p>
          </div>
        </Show>
      </div>
    </div>
  }
}

#[component]
fn AlertItem(
  alert: NetworkAlert,
  #[prop(into)] on_acknowledge: Callback<ev::MouseEvent>,
) -> impl IntoView {
  let (icon, bg_color) = match alert.alert_type {
    AlertType::HighRtt => ("⏱️", "bg-yellow-50 dark:bg-yellow-900/20"),
    AlertType::HighPacketLoss => ("📦", "bg-red-50 dark:bg-red-900/20"),
    AlertType::HighJitter => ("📈", "bg-orange-50 dark:bg-orange-900/20"),
    AlertType::QualityDegradation => ("📉", "bg-yellow-50 dark:bg-yellow-900/20"),
    AlertType::ConnectionUnstable => ("⚠️", "bg-red-50 dark:bg-red-900/20"),
  };

  let i18n = use_i18n();

  let severity_icon = match alert.severity {
    AlertSeverity::Warning => "⚠️",
    AlertSeverity::Critical => "🚨",
  };

  let severity_label = format!("{:?}", alert.severity);
  let acknowledged = alert.acknowledged;

  view! {
    <div class={format!("alert-item flex items-start gap-3 p-3 rounded-lg {bg_color}")}>
      <span class="text-xl">{icon}</span>

      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2">
          <span class="text-sm">{severity_icon}</span>
          <span class="text-xs font-medium text-gray-600 dark:text-gray-300 uppercase">
            {severity_label}
          </span>
        </div>
        <p class="text-sm text-gray-900 dark:text-white mt-1">{alert.message.clone()}</p>
        <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
          {t_string!(i18n, network_peer_prefix).to_string().replace("{}", &alert.peer_id.clone())}
        </p>
      </div>

      <Show when={move || !acknowledged}>
        <button
          class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
          on:click={move |e| on_acknowledge.run(e)}
          title=t_string!(i18n, network_acknowledge)
        >
          "✕"
        </button>
      </Show>
    </div>
  }
}
