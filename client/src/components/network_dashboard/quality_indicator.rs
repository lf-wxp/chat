//! Quality Indicator Components
//!
//! Visual indicators for network quality levels

use crate::network_quality::QualityLevel;
use leptos::prelude::*;

/// Quality level badge component
#[component]
pub fn QualityBadge(quality: QualityLevel) -> impl IntoView {
  let color = quality.color();
  let label = quality.label();

  view! {
    <div
      class="quality-badge inline-flex items-center px-3 py-1 rounded-full text-sm font-medium"
      style={format!("background-color: {color}33; color: {color}")}
    >
      <span class="mr-1">"📶"</span>
      {label}
    </div>
  }
}

/// Reactive quality indicator (signal-based)
#[component]
pub fn QualityIndicator(quality: Signal<QualityLevel>) -> impl IntoView {
  let color_signal = Signal::derive(move || quality.get().color());
  let label_signal = Signal::derive(move || quality.get().label());

  view! {
    <div
      class="quality-indicator flex items-center gap-1 px-2 py-1 rounded-md bg-gray-100 dark:bg-gray-800"
      style:color={color_signal}
    >
      <span>"📶"</span>
      <span class="text-xs font-medium">{label_signal}</span>
    </div>
  }
}
