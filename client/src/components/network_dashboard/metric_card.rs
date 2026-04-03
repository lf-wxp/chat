//! Metric Card Components
//!
//! Card components for displaying network metrics

use leptos::prelude::*;

#[component]
pub fn MetricCard(
  #[prop(into)] label: String,
  value: String,
  icon: &'static str,
  #[prop(optional)] color: Option<&'static str>,
) -> impl IntoView {
  let color = color.unwrap_or("#6b7280");

  view! {
    <div class="metric-card bg-white dark:bg-gray-800 rounded-lg p-4 shadow-sm border border-gray-200 dark:border-gray-700">
      <div class="flex items-center justify-between">
        <span class="text-2xl">{icon}</span>
        <span
          class="text-xl font-bold"
        style={format!("color: {color}")}
        >
          {value}
        </span>
      </div>
      <p class="mt-2 text-xs text-gray-500 dark:text-gray-400">{label}</p>
    </div>
  }
}

#[component]
pub fn StatItem(#[prop(into)] label: String, value: String, icon: &'static str) -> impl IntoView {
  view! {
    <div class="stat-item">
      <div class="flex items-center gap-1 text-xs text-gray-500 dark:text-gray-400">
        <span>{icon}</span>
        <span>{label}</span>
      </div>
      <p class="text-sm font-medium text-gray-900 dark:text-white mt-1">{value}</p>
    </div>
  }
}
