//! File attachment card rendered inside a chat bubble.
//!
//! Displays filename, size, type icon, a live progress bar (for
//! transfers still in flight), and a download button (once the
//! transfer is complete). For inbound transfers the card also shows
//! a "⚠️ Security Risk" label when the file extension is flagged as
//! potentially dangerous (Req 6.8b / 6.8c).

use crate::chat::models::FileRef;
use crate::components::chat_view::dialog::DialogState;
use crate::file_transfer::{
  TransferProgress, TransferStatus, format_bytes, try_use_file_transfer_manager,
};
use crate::i18n;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;
use message::MessageId;
use wasm_bindgen::JsCast;

/// Whether the MIME type represents an image.
fn is_image(mime: &str) -> bool {
  mime.starts_with("image/")
}

/// Build the file-card icon for the given MIME type.
fn type_icon(mime: &str) -> icondata::Icon {
  if is_image(mime) {
    i::LuImage
  } else if mime.starts_with("video/") {
    i::LuFilm
  } else if mime.starts_with("audio/") {
    i::LuMusic
  } else if mime.starts_with("application/pdf") {
    i::LuBookOpen
  } else if mime.contains("zip") || mime.contains("compressed") || mime.contains("tar") {
    i::LuArchive
  } else if mime.starts_with("text/") {
    i::LuFileText
  } else {
    i::LuPaperclip
  }
}

/// File attachment card.
#[component]
pub fn FileCard(
  /// Reference metadata (stable across renders).
  file: FileRef,
  /// Id of the chat message that owns this card.
  message_id: MessageId,
  /// Whether the card belongs to an outgoing message.
  outgoing: bool,
  /// Sender name for accessibility labels. Uses `String` so
  /// dynamic names from `ChatMessage::sender_name` can be passed).
  #[prop(default = String::new())]
  sender_name: String,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let transfers = try_use_file_transfer_manager();

  let file_for_icon = file.clone();
  let file_for_name = file.clone();
  let file_for_title = file.clone();
  let file_for_size = file.clone();
  let file_for_download = file.clone();

  // Pre-compute the alt text so the view closure stays `Fn`.
  let sender_name_for_alt = sender_name.clone();
  let alt_text = Memo::new(move |_| {
    let name = if sender_name_for_alt.is_empty() {
      "unknown".to_string()
    } else {
      sender_name_for_alt.clone()
    };
    format!("Image preview from {}", name)
  });

  // Clone `transfers` for each closure that captures it.
  let transfers_for_download = transfers.clone();
  let transfers_for_thumb = transfers.clone();

  // Pull the live progress + status signals for this message id. We
  // fall back to a static "Completed" snapshot when no transfer is
  // tracked — covers re-hydrated messages after a page refresh.
  let (progress_signal, status_signal): (
    Option<RwSignal<TransferProgress>>,
    Option<RwSignal<TransferStatus>>,
  ) = if let Some(ref mgr) = transfers {
    if outgoing {
      let tx = mgr.get_outbound(&message_id);
      (
        tx.as_ref().map(|t| t.progress),
        tx.as_ref().map(|t| t.status),
      )
    } else {
      let rx = mgr.get_inbound_by_message(&message_id);
      (
        rx.as_ref().map(|r| r.progress),
        rx.as_ref().map(|r| r.status),
      )
    }
  } else {
    (None, None)
  };

  let percent = Memo::new(move |_| progress_signal.map_or(100u8, |p| p.get().percent()));
  let bytes_per_sec = Memo::new(move |_| progress_signal.map_or(0u64, |p| p.get().bytes_per_sec));
  let eta_secs = Memo::new(move |_| progress_signal.and_then(|p| p.get().eta_secs));
  let status = Memo::new(move |_| status_signal.map_or(TransferStatus::Completed, |s| s.get()));

  // Inbound download URL, if any. Only populated after the receiver
  // finishes reassembly + hash verification.
  let download_url = Memo::new(move |_| {
    if outgoing {
      transfers_for_download
        .as_ref()
        .and_then(|mgr| mgr.get_outbound(&message_id))
        .map(|tx| tx.object_url)
    } else {
      transfers_for_download
        .as_ref()
        .and_then(|mgr| mgr.get_inbound_by_message(&message_id))
        .and_then(|rx| rx.object_url.get())
    }
  });

  // Thumbnail URL for image previews (Req 6.7 / P1-6).
  // For outgoing: use the generated 128×128 thumbnail if available,
  // falling back to the full blob URL.
  // For incoming: use the full blob URL (only available after
  // reassembly completes, which is fine since the file is already
  // in memory).
  let thumbnail_url = Memo::new(move |_| {
    if outgoing {
      transfers_for_thumb
        .as_ref()
        .and_then(|mgr| mgr.get_outbound(&message_id))
        .and_then(|tx| tx.thumbnail_url.get())
        .or_else(|| download_url.get())
    } else {
      download_url.get()
    }
  });

  let show_progress = Memo::new(move |_| {
    matches!(
      status.get(),
      TransferStatus::Preparing | TransferStatus::InProgress | TransferStatus::Paused
    )
  });

  // Whether the cancel button should be visible (in-progress,
  // non-terminal outgoing or incoming transfers, P2-8).
  let show_cancel = Memo::new(move |_| {
    matches!(
      status.get(),
      TransferStatus::InProgress | TransferStatus::Preparing | TransferStatus::Paused
    )
  });

  let cancel_label = Memo::new(move |_| {
    if outgoing {
      t_string!(i18n, file.cancel_transfer)
    } else {
      t_string!(i18n, file.cancel_receive)
    }
  });

  // G14 — receiver-side user-initiated pause/resume affordances.
  // Only inbound, non-terminal transfers expose pause/resume; the
  // sender already has a Cancel button which covers the symmetric
  // outbound case.
  let show_pause = Memo::new(move |_| {
    !outgoing
      && matches!(
        status.get(),
        TransferStatus::InProgress | TransferStatus::Preparing
      )
  });
  let show_resume = Memo::new(move |_| !outgoing && matches!(status.get(), TransferStatus::Paused));

  let on_pause = {
    let mgr = try_use_file_transfer_manager();
    Callback::new(move |_: ()| {
      if let Some(ref m) = mgr {
        m.pause_inbound(&message_id);
      }
    })
  };

  let on_resume = {
    let mgr = try_use_file_transfer_manager();
    Callback::new(move |_: ()| {
      if let Some(ref m) = mgr {
        m.resume_inbound(&message_id);
      }
    })
  };

  let status_label = Memo::new(move |_| match status.get() {
    TransferStatus::Preparing => t_string!(i18n, file.preparing).to_string(),
    TransferStatus::InProgress => {
      if outgoing {
        t_string!(i18n, file.uploading).to_string()
      } else {
        t_string!(i18n, file.downloading).to_string()
      }
    }
    TransferStatus::Paused => t_string!(i18n, file.paused).to_string(),
    TransferStatus::Completed => t_string!(i18n, file.completed).to_string(),
    TransferStatus::Cancelled => t_string!(i18n, file.cancelled).to_string(),
    TransferStatus::Failed(reason) => format!("{}: {reason}", t_string!(i18n, file.failed)),
    TransferStatus::HashMismatch => t_string!(i18n, file.hash_mismatch).to_string(),
  });

  // G15 — save-anyway prompt for inbound files flagged dangerous.
  // Captured i18n strings + (optional) DialogState pulled from the
  // reactive owner so the async confirm path can run without
  // re-entering the i18n context (which `spawn_local` futures
  // cannot do safely).
  let save_anyway_msg = {
    let filename = file_for_download.filename.clone();
    format!(
      "{} ({})",
      t_string!(i18n, file.save_anyway_detail),
      filename
    )
  };
  let save_anyway_dialog: Option<DialogState> = use_context::<DialogState>();
  let dangerous_inbound = !outgoing && file_for_download.dangerous;

  let eta_label = Memo::new(move |_| {
    let eta = eta_secs.get();
    let speed = bytes_per_sec.get();
    match (eta, speed) {
      (Some(secs), s) if s > 0 => format!("{} · {}/s", format_eta(secs), format_bytes(s)),
      (None, s) if s > 0 => format!("{}/s", format_bytes(s)),
      _ => String::new(),
    }
  });

  // Re-receive: request retransmission after hash mismatch.
  let on_re_receive = {
    let mgr = try_use_file_transfer_manager();
    let msg_id = message_id;
    Callback::new(move |_: ()| {
      if let Some(ref m) = mgr {
        m.request_resume(&msg_id);
      }
    })
  };

  // Cancel: Callback wrapper so it's Clone + Fn inside the <Show>.
  let on_cancel = {
    let mgr = try_use_file_transfer_manager();
    let is_outgoing = outgoing;
    Callback::new(move |_: ()| {
      if let Some(ref m) = mgr {
        if is_outgoing {
          m.cancel_outbound(&message_id);
        } else {
          m.cancel_inbound(&message_id);
        }
      }
    })
  };

  let mime_for_thumb = file_for_icon.mime_type.clone();
  let show_thumbnail =
    Memo::new(move |_| is_image(&mime_for_thumb) && thumbnail_url.get().is_some());

  view! {
    <div class="message-file" data-testid="message-file" role="group" aria-label=move || t_string!(i18n, file.card_aria)>
      <Show when=move || show_thumbnail.get() fallback=|| ()>
        <img
          class="message-file-preview"
          src=move || thumbnail_url.get().unwrap_or_default()
          alt=move || alt_text.get()
          loading="lazy"
        />
      </Show>
      <div class="message-file-head">
        <span class="message-file-icon" aria-hidden="true"><Icon icon=type_icon(&file_for_icon.mime_type) /></span>
        <div class="message-file-meta">
          <div class="message-file-name" title=file_for_title.filename.clone()>
            {move || {
              let file_info = file_for_name.clone();
              if file_info.dangerous {
                let name = &file_info.filename;
                let ext_start = name.rfind('.').unwrap_or(name.len());
                let (stem, ext) = name.split_at(ext_start);
                view! {
                  {stem.to_string()}
                  <span class="message-file-ext-danger" data-testid="file-ext-danger">{ext.to_string()}</span>
                  <span class="message-file-danger" data-testid="file-danger-badge" title=move || t_string!(i18n, file.security_risk)>
                    " ⚠️ "
                    {t_string!(i18n, file.security_risk)}
                  </span>
                }.into_any()
              } else {
                view! { {file_info.filename.clone()} }.into_any()
              }
            }}
          </div>
          <div class="message-file-size">{format_bytes(file_for_size.size)}</div>
        </div>
      </div>

      <div class="message-file-body">
        <Show when=move || show_progress.get() fallback=|| ()>
          <div class="message-file-progress" role="progressbar"
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow=move || percent.get()
            data-testid="file-progress"
          >
            <div class="message-file-progress-bar" style=move || format!("width: {}%", percent.get())></div>
          </div>
          <div class="message-file-status">
            <span>{move || status_label.get()}</span>
            <span class="message-file-eta">{move || eta_label.get()}</span>
          </div>
        </Show>

        // Cancel button for in-progress outgoing transfers.
        <Show when=move || show_cancel.get() fallback=|| ()>
          <button
            type="button"
            class="message-file-cancel"
            on:click=move |_| on_cancel.run(())
            data-testid="file-cancel"
          >
            {move || cancel_label.get()}
          </button>
        </Show>

        // G14 — receiver-side pause button (in-flight inbound only).
        <Show when=move || show_pause.get() fallback=|| ()>
          <button
            type="button"
            class="message-file-pause"
            on:click=move |_| on_pause.run(())
            data-testid="file-pause"
            aria-label=move || t_string!(i18n, file.pause_transfer)
            title=move || t_string!(i18n, file.pause_transfer)
          >
            <Icon icon=i::LuPause />
            <span class="message-file-action-label">
              {move || t_string!(i18n, file.pause_transfer)}
            </span>
          </button>
        </Show>

        // G14 — receiver-side resume button (paused inbound only).
        <Show when=move || show_resume.get() fallback=|| ()>
          <button
            type="button"
            class="message-file-resume"
            on:click=move |_| on_resume.run(())
            data-testid="file-resume"
            aria-label=move || t_string!(i18n, file.resume_transfer)
            title=move || t_string!(i18n, file.resume_transfer)
          >
            <Icon icon=i::LuPlay />
            <span class="message-file-action-label">
              {move || t_string!(i18n, file.resume_transfer)}
            </span>
          </button>
        </Show>

        // Hash-mismatch warning with a re-receive placeholder button
        // (Req 6.5a: "File may be corrupted, recommend re-receiving").
        <Show
          when=move || matches!(status.get(), TransferStatus::HashMismatch)
          fallback=|| ()
        >
          <div class="message-file-hash-mismatch" data-testid="file-hash-mismatch">
            <span class="message-file-hash-mismatch-text">
              {move || t_string!(i18n, file.hash_mismatch)}
            </span>
            <button
              type="button"
              class="message-file-re-receive"
              on:click=move |_| on_re_receive.run(())
              data-testid="file-re-receive"
            >
              {move || t_string!(i18n, file.re_receive)}
            </button>
          </div>
        </Show>

        // Failed status label (when not a hash-mismatch).
        <Show
          when=move || matches!(status.get(), TransferStatus::Failed(_))
          fallback=|| ()
        >
          <div class="message-file-status message-file-status-failed">
            <span>{move || status_label.get()}</span>
          </div>
        </Show>

        <Show
          when=move || matches!(status.get(), TransferStatus::Completed)
          fallback=|| ()
        >
          {
            let filename_for_download = file_for_download.filename.clone();
            let save_anyway_msg = save_anyway_msg.clone();
            let dialog = save_anyway_dialog.clone();
            let i18n_for_download = i18n;
            move || {
              let url = download_url.get();
              let filename = filename_for_download.clone();
              let save_anyway_msg = save_anyway_msg.clone();
              let dialog = dialog.clone();
              match url {
                Some(href) if dangerous_inbound => {
                  // G15 — receiver-side save-anyway confirm. The
                  // first click opens a dialog; only after the user
                  // confirms do we synthesise a click on a hidden
                  // <a download> to actually persist the blob.
                  let href_for_handler = href.clone();
                  let filename_for_handler = filename.clone();
                  view! {
                    <button
                      type="button"
                      class="message-file-download message-file-download-danger"
                      data-testid="file-download-danger-btn"
                      on:click=move |_| {
                        let dialog = dialog.clone();
                        let msg = save_anyway_msg.clone();
                        let href = href_for_handler.clone();
                        let filename = filename_for_handler.clone();
                        leptos::task::spawn_local(async move {
                          let confirmed = match dialog {
                            Some(d) => d.confirm(msg).await,
                            None => true,
                          };
                          if !confirmed {
                            return;
                          }
                          // Synthesise a click on a transient anchor
                          // so the browser's download path runs.
                          // Use the generic `Element::set_attribute`
                          // surface so we don't need to enable the
                          // `HtmlAnchorElement` web-sys feature
                          // just for two attribute writes.
                          let Some(window) = web_sys::window() else { return };
                          let Some(document) = window.document() else { return };
                          let Ok(anchor) = document.create_element("a") else { return };
                          let _ = anchor.set_attribute("href", &href);
                          let _ = anchor.set_attribute("download", &filename);
                          let _ = anchor.set_attribute("style", "display:none");
                          let Some(body) = document.body() else { return };
                          let _ = body.append_child(&anchor);
                          // Cast to HtmlElement to reach `click()`,
                          // which is part of HTMLElement (already
                          // enabled via the `HtmlElement` feature).
                          if let Some(el) = anchor.dyn_ref::<web_sys::HtmlElement>() {
                            el.click();
                          }
                          let _ = body.remove_child(&anchor);
                        });
                      }
                    >
                      {t_string!(i18n_for_download, file.download)}
                    </button>
                  }
                  .into_any()
                }
                Some(href) => view! {
                  <a
                    class="message-file-download"
                    href=href
                    download=filename
                    data-testid="file-download"
                  >
                    {t_string!(i18n_for_download, file.download)}
                  </a>
                }
                .into_any(),
                None => view! {
                  <span class="message-file-download disabled">
                    {t_string!(i18n_for_download, file.completed)}
                  </span>
                }
                .into_any(),
              }
            }
          }
        </Show>
      </div>
    </div>
  }
}

/// Format an ETA in seconds as `H:MM:SS` / `MM:SS`.
fn format_eta(seconds: u64) -> String {
  crate::utils::format_duration(seconds)
}
