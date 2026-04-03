//! Transfer progress UI components

use std::fmt::Write;

use leptos::prelude::*;
use leptos_i18n::t_string;

use message::transfer::TransferBitmap;

use super::{TransferDirection, TransferManager, TransferStatus};
use crate::{i18n::*, utils};

/// Transfer progress bar component
#[component]
pub fn TransferProgress(
  /// Transfer ID
  #[prop(into)]
  transfer_id: String,
) -> impl IntoView {
  let manager = TransferManager::use_manager();
  let tid = transfer_id.clone();
  let i18n = use_i18n();

  view! {
    <div class="transfer-progress">
      {move || {
        let task = manager.tasks.with_value(|tasks| tasks.get(&tid).cloned());
        if let Some(task) = task {
          let bitmap = TransferBitmap::from_bytes(task.bitmap.clone(), task.meta.total_chunks);
          let progress = bitmap.progress_percent();
          let status_text = match task.status {
            TransferStatus::Pending => t_string!(i18n, transfer_pending),
            TransferStatus::Transferring => t_string!(i18n, transfer_transferring),
            TransferStatus::Paused => t_string!(i18n, transfer_paused),
            TransferStatus::Completed => t_string!(i18n, transfer_completed),
            TransferStatus::Cancelled => t_string!(i18n, transfer_cancelled),
            TransferStatus::Failed => t_string!(i18n, transfer_failed),
          };
          let direction_icon = match task.direction {
            TransferDirection::Sending => "⬆️",
            TransferDirection::Receiving => "⬇️",
          };

          view! {
            <div class="transfer-item">
              <div class="transfer-info">
                <span class="transfer-icon">{direction_icon}</span>
                <span class="transfer-name truncate">{task.meta.file_name.clone()}</span>
                <span class="transfer-size text-xs text-secondary">
                  {utils::format_file_size(task.meta.file_size)}
                </span>
              </div>
              <div class="transfer-bar">
                <div
                  class="transfer-bar-fill"
                  style=format!("width: {}%", progress)
                ></div>
              </div>
              <div class="transfer-meta text-xs text-secondary">
                <span>{status_text}</span>
                <span>{format!("{progress:.0}%")}</span>
              </div>
            </div>
          }.into_any()
        } else {
            let _: () = view! {};
            ().into_any()
        }
      }}
    </div>
  }
}

/// Active transfer panel (displays all ongoing transfer tasks)
#[component]
pub fn ActiveTransferPanel() -> impl IntoView {
  let manager = TransferManager::use_manager();
  let i18n = use_i18n();

  view! {
    <div class="active-transfer-panel">
      {move || {
        let transfers = manager.active_transfers();
        if transfers.is_empty() {
          return view! { <div class="transfer-panel-hidden"></div> }.into_any();
        }

        view! {
          <div class="transfer-panel">
            <div class="transfer-panel-header">
              <span class="transfer-panel-title">{t_string!(i18n, transfer_file_transfers)}</span>
              <span class="transfer-panel-count">{format!("({})", transfers.len())}</span>
            </div>
            <div class="transfer-panel-list">
              {transfers.into_iter().map(|(id, _)| {
                view! { <TransferProgress transfer_id=id /> }
              }).collect_view()}
            </div>
          </div>
        }.into_any()
      }}
    </div>
  }
}

/// File card component (for rendering file messages in chat)
///
/// If the file has an associated transfer ID and the transfer is in progress,
/// displays a progress bar.
#[component]
pub fn FileCard(
  /// File name
  #[prop(into)]
  file_name: String,
  /// File size in bytes
  file_size: u64,
  /// MIME type
  #[prop(into)]
  mime_type: String,
  /// Associated transfer ID (optional)
  #[prop(optional, into)]
  transfer_id: Option<String>,
) -> impl IntoView {
  let i18n = use_i18n();
  let icon = file_icon_for_mime(&mime_type);
  let size_str = utils::format_file_size(file_size);
  let name_for_aria = file_name.clone();
  let name_display = file_name.clone();

  // File extension
  let extension = file_name.rsplit('.').next().unwrap_or("").to_uppercase();

  let manager = TransferManager::use_manager();
  let tid = transfer_id.clone();

  view! {
    <div class="file-card" tabindex=0 aria-label=format!("{}", t_string!(i18n, transfer_file_aria).to_string().replace("{}", &name_for_aria))>
      <div class="file-card-icon-area">
        <span class="file-card-icon">{icon}</span>
        {if extension.is_empty() {
          let _: () = view! {};
          ().into_any()
        } else {
          view! { <span class="file-card-ext">{extension}</span> }.into_any()
        }}
      </div>
      <div class="file-card-body">
        <div class="file-card-name truncate">{name_display}</div>
        <div class="file-card-meta text-xs text-secondary">{size_str}</div>
        // Transfer progress bar (if there is an active transfer)
        {move || {
          let Some(ref transfer_id) = tid else {
            return view! { <div></div> }.into_any();
          };
          let task = manager.tasks.with_value(|tasks| tasks.get(transfer_id).cloned());
          let Some(task) = task else {
            return view! { <div></div> }.into_any();
          };

          // Only show progress bar during transfer/pending/paused
          if !matches!(task.status, TransferStatus::Pending | TransferStatus::Transferring | TransferStatus::Paused) {
            if task.status == TransferStatus::Completed {
              return view! {
                <div class="file-card-status completed">
                  <span class="file-card-status-icon">"✅"</span>
                  <span>{t_string!(i18n, transfer_completed)}</span>
                </div>
              }.into_any();
            }
            if task.status == TransferStatus::Cancelled || task.status == TransferStatus::Failed {
              let status_text = if task.status == TransferStatus::Failed { t_string!(i18n, transfer_failed) } else { t_string!(i18n, transfer_cancelled) };
              return view! {
                <div class="file-card-status failed">
                  <span class="file-card-status-icon">"❌"</span>
                  <span>{status_text}</span>
                </div>
              }.into_any();
            }
            return view! { <div></div> }.into_any();
          }

          let bitmap = TransferBitmap::from_bytes(task.bitmap.clone(), task.meta.total_chunks);
          let progress = bitmap.progress_percent();
          let transferred = utils::format_file_size(task.transferred_bytes);
          let total = utils::format_file_size(task.meta.file_size);
          let direction_icon = match task.direction {
            TransferDirection::Sending => "⬆️",
            TransferDirection::Receiving => "⬇️",
          };
          let status_text = match task.status {
            TransferStatus::Pending => t_string!(i18n, transfer_pending_short),
            TransferStatus::Transferring => t_string!(i18n, transfer_transferring),
            TransferStatus::Paused => t_string!(i18n, transfer_paused),
            _ => "".into(),
          };

          let mut progress_label = String::new();
          let _ = write!(progress_label, "{direction_icon} {status_text} {transferred}/{total} ({progress:.0}%)");

          view! {
            <div class="file-card-progress">
              <div class="file-card-progress-bar">
                <div
                  class="file-card-progress-fill"
                  style=format!("width: {}%", progress)
                ></div>
              </div>
              <div class="file-card-progress-text text-xs text-secondary">
                {progress_label}
              </div>
            </div>
          }.into_any()
        }}
      </div>
    </div>
  }
}

/// Returns file icon based on MIME type
fn file_icon_for_mime(mime_type: &str) -> &'static str {
  if mime_type.starts_with("image/") {
    "🖼️"
  } else if mime_type.starts_with("video/") {
    "🎬"
  } else if mime_type.starts_with("audio/") {
    "🎵"
  } else if mime_type.contains("pdf") {
    "📄"
  } else if mime_type.contains("zip")
    || mime_type.contains("rar")
    || mime_type.contains("tar")
    || mime_type.contains("gz")
  {
    "📦"
  } else if mime_type.contains("text") || mime_type.contains("json") || mime_type.contains("xml") {
    "📝"
  } else if mime_type.contains("spreadsheet") || mime_type.contains("excel") {
    "📊"
  } else if mime_type.contains("presentation") || mime_type.contains("powerpoint") {
    "📽️"
  } else if mime_type.contains("document") || mime_type.contains("word") {
    "📃"
  } else {
    "📎"
  }
}
