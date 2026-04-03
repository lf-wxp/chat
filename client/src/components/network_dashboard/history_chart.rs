//! History Chart Components
//!
//! Visualizations for network history data

use crate::i18n::*;
use crate::network_quality::PeerNetworkHistory;
use leptos::prelude::*;
use leptos_i18n::t_string;

#[component]
pub fn HistoryChart(
  history: Signal<Option<PeerNetworkHistory>>,
  #[prop(default = "rtt")] metric: &'static str,
) -> impl IntoView {
  let data_signal = Signal::derive(move || {
    history
      .get()
      .map(|h| {
        h.history
          .iter()
          .enumerate()
          .map(|(idx, point)| {
            let value = match metric {
              "rtt" => point.rtt_ms,
              "loss" => point.packet_loss_percent,
              "bitrate" => point.bitrate_kbps,
              "jitter" => point.jitter_ms,
              _ => point.rtt_ms,
            };
            (value, idx)
          })
          .collect::<Vec<_>>()
      })
      .unwrap_or_default()
  });

  let max_signal = Signal::derive(move || {
    data_signal
      .get()
      .iter()
      .map(|(v, _)| *v)
      .fold(0.0_f64, f64::max)
      .max(1.0)
  });

  let i18n = use_i18n();
  let metric_label = match metric {
    "rtt" => t_string!(i18n, network_rtt_unit).to_string(),
    "loss" => t_string!(i18n, network_packet_loss_unit).to_string(),
    "bitrate" => t_string!(i18n, network_bitrate_unit).to_string(),
    "jitter" => t_string!(i18n, network_jitter_unit).to_string(),
    _ => t_string!(i18n, network_value).to_string(),
  };

  view! {
    <div class="history-chart">
      <div class="flex items-end justify-between h-16 gap-0.5 bg-gray-50 dark:bg-gray-900 rounded p-2">
        <For each={move || data_signal.get()} key={|(_, idx)| *idx} let:item>
          {
            let value = item.0;
            let max_val = max_signal.get();
            let height_percent = if max_val > 0.0 {
              (value / max_val * 100.0).min(100.0)
            } else {
              0.0
            };

            view! {
              <div
                class="flex-1 bg-blue-500 rounded-t transition-all duration-200 hover:bg-blue-400"
                style:min-height="2px"
                style:height={format!("{height_percent}%")}
                title={format!("{value:.1}")}
              />
            }
          }
        </For>
      </div>
      <p class="text-xs text-gray-400 mt-1">{metric_label}</p>
    </div>
  }
}

#[component]
pub fn QualityDistribution(history: Signal<Option<PeerNetworkHistory>>) -> impl IntoView {
  let i18n = use_i18n();
  let distribution_signal = Signal::derive(move || {
    history
      .get()
      .map_or((0, 0, 0, 0), |h| h.quality_distribution())
  });

  let total_signal = Signal::derive(move || {
    let (e, g, f, p) = distribution_signal.get();
    e + g + f + p
  });

  view! {
    <div class="quality-distribution mt-4">
      <h4 class="text-xs font-medium text-gray-500 dark:text-gray-400 mb-2">
        {t_string!(i18n, network_quality_distribution)}
      </h4>
      <div class="flex items-center gap-2">
        <QualityBar
          label=t_string!(i18n, network_excellent).to_string()
          count={Signal::derive(move || distribution_signal.get().0)}
          total={total_signal}
          color="#22c55e"
        />
        <QualityBar
          label=t_string!(i18n, network_good).to_string()
          count={Signal::derive(move || distribution_signal.get().1)}
          total={total_signal}
          color="#84cc16"
        />
        <QualityBar
          label=t_string!(i18n, network_fair).to_string()
          count={Signal::derive(move || distribution_signal.get().2)}
          total={total_signal}
          color="#eab308"
        />
        <QualityBar
          label=t_string!(i18n, network_poor).to_string()
          count={Signal::derive(move || distribution_signal.get().3)}
          total={total_signal}
          color="#ef4444"
        />
      </div>
    </div>
  }
}

#[component]
fn QualityBar(
  #[prop(into)] label: String,
  count: Signal<u32>,
  total: Signal<u32>,
  color: &'static str,
) -> impl IntoView {
  let percent_signal = Signal::derive(move || {
    let t = total.get();
    if t > 0 {
      (count.get() as f64 / t as f64 * 100.0) as u32
    } else {
      0
    }
  });

  view! {
    <div class="quality-bar flex-1 text-center">
      <div
        class="h-2 rounded transition-all duration-300"
        style:background-color={color}
        style:width={move || format!("{}%", percent_signal.get().max(5))}
      />
      <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">{count}</p>
      <p class="text-xs text-gray-400">{label}</p>
    </div>
  }
}
