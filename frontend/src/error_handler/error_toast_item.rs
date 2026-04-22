//! Error toast notification component.
//!
//! Renders individual error toast notifications with expandable details
//! and i18n-aware messages.

use crate::error_handler::ErrorToast;
use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};

/// Resolve an error message using the i18n key with compile-time `t_string!`
/// dispatch. Falls back to the default English message when the key is
/// not recognised (e.g. keys added server-side before the client is updated).
///
/// Template variables (e.g. `{{reason}}`) are substituted using the `fallback`
/// parameter for keys that expect dynamic content.
fn resolve_error_message(key: &str, fallback: &str) -> String {
  let i18n = i18n::use_i18n();
  let resolved: &str = match key {
    // ── Signaling ──
    "error.sig001" => t_string!(i18n, error.sig001),
    "error.sig002" => t_string!(i18n, error.sig002),
    "error.sig003" => t_string!(i18n, error.sig003),
    "error.sig101" => t_string!(i18n, error.sig101),
    // ── Chat ──
    "error.cht001" => t_string!(i18n, error.cht001),
    "error.cht101" => t_string!(i18n, error.cht101),
    "error.cht102" => t_string!(i18n, error.cht102),
    "error.cht103" => t_string!(i18n, error.cht103),
    "error.cht104" => t_string!(i18n, error.cht104),
    "error.cht105" => t_string!(i18n, error.cht105),
    // ── Audio/Video ──
    "error.av001" => t_string!(i18n, error.av001),
    "error.av201" => t_string!(i18n, error.av201),
    "error.av401" => t_string!(i18n, error.av401),
    "error.av402" => t_string!(i18n, error.av402),
    "error.av403" => t_string!(i18n, error.av403),
    // ── Room ──
    "error.rom001" => t_string!(i18n, error.rom001),
    "error.rom101" => t_string!(i18n, error.rom101),
    "error.rom102" => t_string!(i18n, error.rom102),
    "error.rom103" => t_string!(i18n, error.rom103),
    "error.rom104" => t_string!(i18n, error.rom104),
    "error.rom105" => t_string!(i18n, error.rom105),
    "error.rom106" => t_string!(i18n, error.rom106),
    "error.rom107" => t_string!(i18n, error.rom107),
    "error.rom108" => t_string!(i18n, error.rom108),
    // ── E2EE ──
    "error.e2e001" => t_string!(i18n, error.e2e001),
    "error.e2e501" => t_string!(i18n, error.e2e501),
    "error.e2e502" => t_string!(i18n, error.e2e502),
    // ── File Transfer ──
    "error.fil001" => t_string!(i18n, error.fil001),
    "error.fil101" => t_string!(i18n, error.fil101),
    "error.fil102" => t_string!(i18n, error.fil102),
    "error.fil201" => t_string!(i18n, error.fil201),
    // ── Theater ──
    "error.thr001" => t_string!(i18n, error.thr001),
    "error.thr101" => t_string!(i18n, error.thr101),
    "error.thr102" => t_string!(i18n, error.thr102),
    "error.thr103" => t_string!(i18n, error.thr103),
    "error.thr104" => t_string!(i18n, error.thr104),
    // ── Auth ──
    "error.auth001" => t_string!(i18n, error.auth001),
    "error.auth501" => t_string!(i18n, error.auth501),
    "error.auth502" => t_string!(i18n, error.auth502),
    "auth.failure_generic" => {
      // Append the server-supplied reason to the localized prefix.
      let prefix = t_string!(i18n, auth.failure_generic);
      return format!("{}: {}", prefix, fallback);
    }
    "auth.session_invalidated" => t_string!(i18n, auth.session_invalidated),
    // ── Persistence ──
    "error.pst001" => t_string!(i18n, error.pst001),
    "error.pst101" => t_string!(i18n, error.pst101),
    // ── System ──
    "error.sys001" => t_string!(i18n, error.sys001),
    "error.sys201" => t_string!(i18n, error.sys201),
    "error.sys301" => t_string!(i18n, error.sys301),
    // ── Generic / UI ──
    "error.connection_lost" => t_string!(i18n, error.connection_lost),
    "error.reconnecting" => t_string!(i18n, error.reconnecting),
    "error.connection_failed" => t_string!(i18n, error.connection_failed),
    "error.auth_failed" => t_string!(i18n, error.auth_failed),
    "error.session_expired" => t_string!(i18n, error.session_expired),
    "error.rate_limit" => t_string!(i18n, error.rate_limit),
    "error.unknown" => t_string!(i18n, error.unknown),
    "error.server_restarted" => t_string!(i18n, error.server_restarted),
    _ => fallback,
  };
  resolved.to_string()
}

/// Single error toast notification component.
#[component]
pub fn ErrorToastItem(
  /// The error toast data.
  toast: ErrorToast,
) -> impl IntoView {
  let manager = use_error_toast_manager();
  let id = toast.id;

  // Compute boolean flags before moving data into signals
  let has_details_entries_val = !toast.details.is_empty();
  let has_trace_val = !toast.trace_id.is_empty();
  let has_detail_key_val = !toast.detail_i18n_key.is_empty();

  // Store toast data in signals for reactivity
  let code = RwSignal::new(toast.code);
  // Store the i18n key and fallback message so the localized text can
  // be resolved reactively inside a view closure (where the i18n signal
  // is properly tracked). This avoids the "outside reactive tracking
  // context" warning that occurs when calling t_string! at component
  // init time.
  let i18n_key = RwSignal::new(toast.i18n_key);
  let fallback_message = RwSignal::new(toast.message);
  let detail_i18n_key = RwSignal::new(toast.detail_i18n_key);
  let has_details = RwSignal::new(has_detail_key_val || has_details_entries_val);
  let details = RwSignal::new(toast.details);
  let trace_id = RwSignal::new(toast.trace_id);
  let is_expanded = RwSignal::new(toast.expanded);

  let has_details_entries = RwSignal::new(has_details_entries_val);
  let has_trace = RwSignal::new(has_trace_val);

  let dismiss_action = move || {
    manager.dismiss(id);
  };

  let toggle_action = move || {
    manager.toggle_expand(id);
    is_expanded.update(|e| *e = !*e);
  };

  // W4 fix: When this toast component is unmounted (e.g. the parent
  // ErrorToastContainer is removed or the page navigates away), we must
  // cancel the pending auto-remove timer so the JS closure is released
  // and does not try to mutate a dropped signal. This is done via
  // Leptos' on_cleanup hook which runs when the component's reactive
  // scope is disposed.
  //
  // Note: The parent ErrorToastContainer also calls `manager.clear_all()`
  // on unmount, which removes all toasts and cancels their timers. This
  // per-item dismiss is a defensive fallback that is safe even when the
  // toast was already cleared — `dismiss()` is a no-op for unknown IDs.
  // Trade-off: toasts are silently removed on navigation, so the user
  // may miss critical errors. This is acceptable to prevent use-after-free.
  leptos::prelude::on_cleanup(move || {
    manager.dismiss(id);
  });

  view! {
    <div class="error-toast" role="alert" aria-live="assertive">
      <div class="error-toast-header">
        <span class="error-toast-code">{move || code.get()}</span>
        <button
          class="error-toast-close"
          on:click=move |_| dismiss_action()
          aria-label="Close error notification"
        >
          "×"
        </button>
      </div>
      <p class="error-toast-message">{move || resolve_error_message(&i18n_key.get(), &fallback_message.get())}</p>
      <Show when=move || has_details.get()>
        <button
          class="error-toast-learn-more"
          on:click=move |_| toggle_action()
        >
          <Show
            when=move || is_expanded.get()
            fallback=move || view! { <span>{t!(i18n::use_i18n(), common.more)}</span> }.into_any()
          >
            <span>{t!(i18n::use_i18n(), common.less)}</span>
          </Show>
        </button>
        <Show when=move || is_expanded.get()>
          <div class="error-toast-details">
            <Show when=move || !detail_i18n_key.get().is_empty()>
              {move || {
                let text = resolve_error_message(&detail_i18n_key.get(), "");
                (!text.is_empty()).then(|| view! { <p class="error-toast-detail-text">{text}</p> })
              }}
            </Show>
            <Show when=move || has_details_entries.get()>
              <dl class="error-toast-detail-list">
                {move || details.get().into_iter().map(|(key, value)| {
                  view! {
                    <>
                      <dt>{key}</dt>
                      <dd>{value}</dd>
                    </>
                  }
                }).collect::<Vec<_>>()}
              </dl>
            </Show>
            <Show when=move || has_trace.get()>
              <p class="error-toast-trace">"Trace: "{move || trace_id.get()}</p>
            </Show>
          </div>
        </Show>
      </Show>
    </div>
  }
}
