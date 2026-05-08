//! Data management section (clear history, cache, export, diagnostics).
//!
//! Pure view + handler wiring — everything testable lives in
//! `data_management_helpers` so this file stays focused on
//! presentation and state wiring.

use super::data_management_helpers::{
  clear_all_history, clear_cache_storage, collect_blacklist, collect_contacts,
  collect_messages_for_export, format_storage_estimate, refresh_storage_estimate,
  timestamped_filename, trigger_download,
};
use crate::blacklist::use_blacklist_state;
use crate::chat::use_chat_manager;
use crate::components::debug::DebugPanelVisibility;
use crate::components::room::confirm_dialog::{ConfirmDialog, ConfirmTone};
use crate::i18n;
use crate::logging::{download_diagnostic_report, use_logger_state};
use crate::settings::{ExportPayload, use_settings_state};
use crate::state::use_app_state;
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;

/// Which destructive action the user is currently being asked to
/// confirm. `None` means no dialog is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingConfirm {
  ClearHistory,
  ClearCache,
}

/// Data management section.
#[component]
pub fn DataManagementSection() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let settings = use_settings_state();
  let app_state = use_app_state();
  let chat = use_chat_manager();
  let logger = use_logger_state();
  let blacklist = use_blacklist_state();

  // Storage estimate ("42.3 MB used of 1.2 GB"), polled lazily.
  let storage_usage: RwSignal<Option<(u64, u64)>> = RwSignal::new(None);
  let status_message: RwSignal<Option<String>> = RwSignal::new(None);
  // Whether an export is currently in progress (disables buttons and
  // shows a spinner overlay on the export row).
  let exporting: RwSignal<bool> = RwSignal::new(false);

  // Tracks which destructive action (if any) is awaiting user
  // confirmation via the inline ConfirmDialog. Replaces the blocking
  // native `window.confirm` (V2-S-2).
  let pending: RwSignal<Option<PendingConfirm>> = RwSignal::new(None);

  // Load the initial storage estimate.
  Effect::new(move |_| {
    refresh_storage_estimate(storage_usage);
  });

  // -------------------- Handlers --------------------

  // Opening handlers just flip the pending-action state; the actual
  // work runs from `on_confirm` below. `i18n` strings are now read
  // inside closures via `t_string!` instead of being eagerly cloned
  // once per render (V2-Q-1).
  let open_clear_history = move |_| pending.set(Some(PendingConfirm::ClearHistory));
  let open_clear_cache = move |_| pending.set(Some(PendingConfirm::ClearCache));

  let on_confirm = {
    let chat = chat.clone();
    Callback::new(move |()| {
      let Some(action) = pending.get_untracked() else {
        return;
      };
      pending.set(None);
      match action {
        PendingConfirm::ClearHistory => {
          let chat = chat.clone();
          let app_state = app_state;
          let status_for = status_message;
          // Snapshot i18n strings before spawning the async task to
          // avoid reading a stale locale if the user switches
          // language while the clear operation is in flight (B-4).
          let success_template = t_string!(i18n, settings.clear_chat_history_success).to_string();
          let failed_msg = t_string!(i18n, settings.clear_chat_history_failed).to_string();
          spawn_local(async move {
            match clear_all_history(&chat, app_state).await {
              Ok(count) => {
                let msg = success_template.replace("{count}", &count.to_string());
                status_for.set(Some(msg));
                refresh_storage_estimate(storage_usage);
              }
              Err(err) => {
                status_for.set(Some(format!("{failed_msg}: {err}")));
              }
            }
          });
        }
        PendingConfirm::ClearCache => {
          clear_cache_storage();
          status_message.set(Some(
            t_string!(i18n, settings.clear_cache_success).to_string(),
          ));
          refresh_storage_estimate(storage_usage);
        }
      }
    })
  };

  let on_cancel = Callback::new(move |()| pending.set(None));

  // Confirm-dialog derived signals. Read the current pending action
  // and map it to title/body/button copy.
  let dialog_title = Signal::derive(move || match pending.get() {
    Some(PendingConfirm::ClearHistory) => t_string!(i18n, settings.clear_chat_history).to_string(),
    Some(PendingConfirm::ClearCache) => t_string!(i18n, settings.clear_cache).to_string(),
    None => String::new(),
  });
  let dialog_desc = Signal::derive(move || match pending.get() {
    Some(PendingConfirm::ClearHistory) => {
      t_string!(i18n, settings.clear_chat_history_confirm).to_string()
    }
    Some(PendingConfirm::ClearCache) => t_string!(i18n, settings.clear_cache_confirm).to_string(),
    None => String::new(),
  });
  let dialog_confirm_label = Signal::derive(move || match pending.get() {
    Some(PendingConfirm::ClearHistory) => t_string!(i18n, settings.clear_chat_history).to_string(),
    Some(PendingConfirm::ClearCache) => t_string!(i18n, settings.clear_cache).to_string(),
    None => String::new(),
  });

  /// Which format to export — used by the shared export helper to
  /// avoid duplicating the async payload-building logic (P2-10).
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  enum ExportFormat {
    Json,
    Html,
  }

  /// Shared export handler — collects data, builds the payload, and
  /// triggers a download in the requested format.
  fn run_export(
    fmt: ExportFormat,
    chat: &crate::chat::ChatManager,
    blacklist: &crate::blacklist::BlacklistState,
    settings: crate::settings::SettingsState,
    app_state: crate::state::AppState,
    exporting: RwSignal<bool>,
  ) {
    exporting.set(true);
    let settings_snapshot = settings.get();
    let contacts = collect_contacts(app_state);
    let blacklist_json = collect_blacklist(blacklist);
    let chat = chat.clone();
    spawn_local(async move {
      let messages = collect_messages_for_export(&chat, app_state).await;
      let payload = ExportPayload::full(
        settings_snapshot,
        messages,
        Some(contacts),
        Some(blacklist_json),
      );
      match fmt {
        ExportFormat::Json => {
          let filename = timestamped_filename("chat-export", "json");
          trigger_download(&filename, "application/json", &payload.to_json());
        }
        ExportFormat::Html => {
          let filename = timestamped_filename("chat-export", "html");
          trigger_download(&filename, "text/html", &payload.to_html());
        }
      }
      exporting.set(false);
    });
  }

  let on_export_json = {
    let chat = chat.clone();
    let blacklist = blacklist.clone();
    move |_| {
      run_export(
        ExportFormat::Json,
        &chat,
        &blacklist,
        settings,
        app_state,
        exporting,
      );
    }
  };
  let on_export_html = {
    let chat = chat.clone();
    let blacklist = blacklist.clone();
    move |_| {
      run_export(
        ExportFormat::Html,
        &chat,
        &blacklist,
        settings,
        app_state,
        exporting,
      );
    }
  };

  // "Open Debug Panel" handler. `DebugPanelVisibility` is provided at
  // the `App` root so in production the context lookup always
  // succeeds; we still defensively surface a status message if a
  // test harness forgets to install it (V2-S-3).
  let on_open_debug = move |_| {
    if let Some(vis) = use_context::<DebugPanelVisibility>() {
      vis.0.set(true);
    } else {
      status_message.set(Some(
        t_string!(i18n, settings.debug_logs_unavailable).to_string(),
      ));
    }
  };
  let on_diag_report = move |_| {
    download_diagnostic_report(&logger, &app_state);
  };

  view! {
    <section class="settings-section" aria-labelledby="data-heading">
      <h2 id="data-heading" class="settings-section-title">
        <Icon icon=i::LuDatabase attr:class="settings-section-icon" />
        {t!(i18n, settings.data_management)}
      </h2>

      // Cache size readout
      <div class="settings-row settings-cache-row">
        <span class="settings-label">{t!(i18n, settings.cache_size_label)}</span>
        <span class="settings-cache-value" data-testid="cache-usage">
          {move || format_storage_estimate(storage_usage.get(), t_string!(i18n, settings.cache_size_unknown))}
        </span>
      </div>

      // Retention policy
      <div class="settings-row">
        <label class="settings-label" for="settings-retention">
          {t!(i18n, settings.retention)}
        </label>
        <select
          id="settings-retention"
          class="settings-select"
          prop:value=move || {
            let r = settings.get().retention;
            crate::persistence::RetentionPolicy::as_str(r)
          }
          on:change=move |ev| {
            let value = event_target_value(&ev);
            if let Some(policy) = crate::persistence::RetentionPolicy::parse_policy(&value) {
              settings.update(|s| s.retention = policy);
            }
          }
        >
          <option value="24h">{t!(i18n, settings.retention_day)}</option>
          <option value="72h">{t!(i18n, settings.retention_three_days)}</option>
          <option value="7d">{t!(i18n, settings.retention_week)}</option>
        </select>
        <p class="settings-hint">{t!(i18n, settings.retention_hint)}</p>
      </div>

      // Clear chat history
      <div class="settings-row">
        <button
          class="btn-secondary settings-action"
          on:click=open_clear_history
          data-testid="clear-chat-history"
        >
          <Icon icon=i::LuTrash2 />
          <span>{t!(i18n, settings.clear_chat_history)}</span>
        </button>
        <p class="settings-hint">{t!(i18n, settings.clear_chat_history_hint)}</p>
      </div>

      // Clear cache
      <div class="settings-row">
        <button
          class="btn-secondary settings-action"
          on:click=open_clear_cache
          data-testid="clear-cache"
        >
          <Icon icon=i::LuBrush />
          <span>{t!(i18n, settings.clear_cache)}</span>
        </button>
      </div>

      // Export data
      <div class="settings-row">
        <span class="settings-label">{t!(i18n, settings.export_data)}</span>
        <div class="settings-button-row">
          <button
            class="btn-secondary settings-action"
            disabled=move || exporting.get()
            on:click=on_export_json
            data-testid="export-json"
          >
            <Icon icon=i::LuFileJson />
            <span>{t!(i18n, settings.export_json)}</span>
          </button>
          <button
            class="btn-secondary settings-action"
            disabled=move || exporting.get()
            on:click=on_export_html
            data-testid="export-html"
          >
            <Icon icon=i::LuFileText />
            <span>{t!(i18n, settings.export_html)}</span>
          </button>
        </div>
      </div>
      <Show when=move || exporting.get()>
        <p class="settings-hint" aria-live="polite">
          <Icon icon=i::LuLoaderCircle attr:class="settings-spinner" />
          {t!(i18n, settings.exporting_data)}
        </p>
      </Show>
      // Security warning for exported data (Req 13.5.8).
      <p class="settings-hint settings-warning-hint">
        <Icon icon=i::LuShieldAlert attr:class="settings-warning-icon" />
        {t!(i18n, settings.export_security_warning)}
      </p>

      // Debug logs
      <div class="settings-row">
        <button
          class="btn-ghost settings-action"
          on:click=on_open_debug
          data-testid="open-debug-panel"
        >
          <Icon icon=i::LuTerminal />
          <span>{t!(i18n, settings.debug_logs_open)}</span>
        </button>
        <p class="settings-hint">{t!(i18n, settings.debug_logs_hint)}</p>
      </div>

      // Diagnostic report
      <div class="settings-row">
        <button
          class="btn-ghost settings-action"
          on:click=on_diag_report
          data-testid="generate-diagnostic-report"
        >
          <Icon icon=i::LuClipboardList />
          <span>{t!(i18n, settings.diagnostic_report)}</span>
        </button>
        <p class="settings-hint">{t!(i18n, settings.diagnostic_report_hint)}</p>
      </div>

      // Operation feedback
      <Show when=move || status_message.get().is_some()>
        <p class="settings-status" aria-live="polite" data-testid="settings-status">
          {move || status_message.get().unwrap_or_default()}
        </p>
      </Show>

      // Inline confirmation dialog replaces the blocking native
      // `window.confirm` (V2-S-2). Mounting is gated by the
      // pending-action signal so no overlay is emitted until the
      // user actually clicks a destructive action button.
      <Show when=move || pending.get().is_some()>
        <ConfirmDialog
          title=dialog_title
          description=dialog_desc
          confirm_label=dialog_confirm_label
          tone=Signal::derive(|| ConfirmTone::Destructive)
          on_confirm=on_confirm
          on_cancel=on_cancel
        />
      </Show>
    </section>
  }
}
