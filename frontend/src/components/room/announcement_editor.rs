//! Announcement editor modal (Req 15.2 §10, §16).
//!
//! Rich-text lite editor supporting Markdown-style **bold**, *italic*
//! and [links](url). Provides live character counter and a preview
//! pane next to the textarea so the owner can verify the result
//! before broadcasting.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::error::validation::max_lengths::ANNOUNCEMENT;

use crate::components::room::confirm_dialog::{ConfirmDialog, ConfirmTone};
use crate::components::room::modal_wrapper::{ModalSize, ModalWrapper};
use crate::i18n;
use icondata as i;
use leptos_icons::Icon;

/// Announcement editor modal.
#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn AnnouncementEditor(
  /// Initial announcement text. Empty for a fresh announcement.
  #[prop(into)]
  initial: Signal<String>,
  /// Called with the new announcement text when the user saves. An
  /// empty string means the announcement should be cleared
  /// (Req 15.2 §14).
  on_save: Callback<String>,
  /// Called when the user cancels.
  on_cancel: Callback<()>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let content = RwSignal::new(initial.get_untracked());
  let error = RwSignal::new(Option::<String>::None);
  let confirm_delete = RwSignal::new(false);

  // Sync local state whenever the initial prop changes (e.g. a new
  // announcement is fetched while the modal is still mounted).
  Effect::new(move |_| {
    content.set(initial.get());
  });

  let length = Memo::new(move |_| content.with(|c| c.chars().count()));
  let over_limit = Memo::new(move |_| length.get() > ANNOUNCEMENT);
  let initial_has_content = Memo::new(move |_| initial.with(|s| !s.is_empty()));

  let handle_submit = move || {
    let current = content.get();
    if current.chars().count() > ANNOUNCEMENT {
      error.set(Some(
        t_string!(i18n, room.announcement_too_long).to_string(),
      ));
      return;
    }
    error.set(None);
    on_save.run(current);
  };

  let on_delete_confirm = Callback::new(move |()| {
    confirm_delete.set(false);
    on_save.run(String::new());
  });
  let on_delete_cancel = Callback::new(move |()| confirm_delete.set(false));

  view! {
    <ModalWrapper
      on_close=on_cancel
      size=ModalSize::Large
      class="announcement-editor"
      labelled_by="announcement-editor-title"
      testid="announcement-editor"
    >
      <header class="modal-header">
        <h2 id="announcement-editor-title" class="modal-title">{t!(i18n, room.announcement)}</h2>
        <button
          type="button"
          class="modal-close"
          aria-label=move || t_string!(i18n, common.close)
          on:click=move |_| on_cancel.run(())
        ><Icon icon=i::LuX /></button>
      </header>
      <div class="modal-body announcement-editor__body">
        <div
          class="announcement-editor__toolbar"
          role="toolbar"
          aria-label=move || t_string!(i18n, room.announcement_toolbar)
        >
          <button
            type="button"
            class="btn btn--ghost announcement-editor__tool"
            title="Bold"
            on:click=move |_| content.update(|c| c.push_str("**bold**"))
          ><Icon icon=i::LuBold /></button>
          <button
            type="button"
            class="btn btn--ghost announcement-editor__tool"
            title="Italic"
            on:click=move |_| content.update(|c| c.push_str("*italic*"))
          ><Icon icon=i::LuItalic /></button>
          <button
            type="button"
            class="btn btn--ghost announcement-editor__tool"
            title="Link"
            on:click=move |_| content.update(|c| c.push_str("[text](https://)"))
          ><Icon icon=i::LuLink /></button>
        </div>
        <div class="announcement-editor__split">
          <textarea
            class="input announcement-editor__textarea"
            rows="8"
            aria-label=move || t_string!(i18n, room.announcement)
            prop:value=move || content.get()
            on:input=move |ev| content.set(event_target_value(&ev))
            data-testid="announcement-editor-textarea"
          />
          <div
            class="announcement-editor__preview"
            aria-label=move || t_string!(i18n, room.announcement_preview)
            data-testid="announcement-editor-preview"
          >
            {move || render_preview_view(&content.get())}
          </div>
        </div>
        <div class="announcement-editor__footer">
          <span
            class=move || {
              if over_limit.get() {
                "announcement-editor__counter announcement-editor__counter--over"
              } else {
                "announcement-editor__counter"
              }
            }
            aria-live="polite"
          >
            {move || format!("{} / {}", length.get(), ANNOUNCEMENT)}
          </span>
          <Show when=move || error.get().is_some()>
            <p class="announcement-editor__error" role="alert">
              {move || error.get().unwrap_or_default()}
            </p>
          </Show>
        </div>
      </div>
      <footer class="modal-footer">
        // Delete button is exposed only when there is an existing
        // announcement so users have an explicit affordance for
        // Req 15.2 §14 (clear announcement) instead of having to
        // empty the textarea by hand.
        <Show when=move || initial_has_content.get()>
          <button
            type="button"
            class="btn btn--danger announcement-editor__delete"
            on:click=move |_| confirm_delete.set(true)
            data-testid="announcement-editor-delete"
          >
            {t!(i18n, room.announcement_delete)}
          </button>
        </Show>
        <button
          type="button"
          class="btn btn--ghost"
          on:click=move |_| on_cancel.run(())
          data-testid="announcement-editor-cancel"
        >
          {t!(i18n, common.cancel)}
        </button>
        <button
          type="button"
          class="btn btn--primary"
          disabled=move || over_limit.get()
          on:click=move |_| handle_submit()
          data-testid="announcement-editor-save"
        >
          {t!(i18n, common.save)}
        </button>
      </footer>
    </ModalWrapper>

    <Show when=move || confirm_delete.get()>
      <ConfirmDialog
        title=Signal::derive(move || t_string!(i18n, room.announcement_delete).to_string())
        description=Signal::derive(move ||
          t_string!(i18n, room.announcement_confirm_delete).to_string())
        confirm_label=Signal::derive(move || t_string!(i18n, room.announcement_delete).to_string())
        tone=Signal::derive(|| ConfirmTone::Destructive)
        on_confirm=on_delete_confirm
        on_cancel=on_delete_cancel
      />
    </Show>
  }
}

/// Render a very small Markdown-ish preview: **bold**, *italic* and
/// [text](url) links. XSS is mitigated by HTML-escaping the raw text
/// before substitution.
///
/// This is intentionally lightweight — the server already persists the
/// content verbatim and the chat UI (Task 16) performs full-featured
/// Markdown rendering elsewhere.
fn render_preview_view(raw: &str) -> impl leptos::IntoView + use<> {
  let html = render_preview_html(raw);
  view! { <div inner_html=html /> }
}

/// Build the preview HTML from a raw announcement string. Split out
/// from the component so it can be unit-tested without a DOM.
pub(super) fn render_preview_html(raw: &str) -> String {
  let escaped = escape_html(raw);
  inline_format(&escaped)
}

/// Apply both bold and italic formatting plus link replacement in a
/// single pass-friendly pipeline. Exposed at `pub(super)` so the
/// unit tests in `tests.rs` can verify the nested cases directly.
pub(super) fn inline_format(input: &str) -> String {
  let bold = delim_replace(input, "**", "<strong>", "</strong>");
  let italic = delim_replace(&bold, "*", "<em>", "</em>");
  replace_links(&italic)
}

/// HTML-escape the minimum subset required for safe embedding inside
/// a `<div inner_html>` rendering path.
pub(super) fn escape_html(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  for ch in input.chars() {
    match ch {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '"' => out.push_str("&quot;"),
      '\'' => out.push_str("&#39;"),
      '\n' => out.push_str("<br>"),
      _ => out.push(ch),
    }
  }
  out
}

/// Replace a paired delimiter such as `**` or `*` with `open` and
/// `close` HTML tags. Implements a small token-based scanner so that
/// nested combinations like `**bold *italic* bold**` work correctly:
/// the function only replaces top-level pairs of `delim` and leaves
/// inner content untouched (other passes will recurse on it).
///
/// Unclosed delimiters are stripped (the surrounding text survives)
/// rather than re-emitted: this keeps the rendered preview clean and
/// avoids leaking bare `**` characters that confuse downstream Markdown
/// rendering.
fn delim_replace(input: &str, delim: &str, open: &str, close: &str) -> String {
  let bytes = input.as_bytes();
  let dlen = delim.len();

  // First pass: count matched pairs to avoid emitting half-tags.
  let mut count = 0_usize;
  let mut probe = 0_usize;
  while probe + dlen <= bytes.len() {
    if &bytes[probe..probe + dlen] == delim.as_bytes() {
      count += 1;
      probe += dlen;
    } else {
      let next = input[probe..].chars().next().map_or(0, |c| c.len_utf8());
      probe += next.max(1);
    }
  }
  let pairs = count / 2;
  if pairs == 0 {
    return input.to_string();
  }

  // Second pass: emit tags for `pairs * 2` delimiter occurrences and
  // drop any trailing dangling delimiter as plain text.
  let mut result = String::with_capacity(input.len());
  let mut i = 0_usize;
  let mut emitted_pairs = 0_usize;
  let mut inside = false;
  while i + dlen <= bytes.len() {
    if &bytes[i..i + dlen] == delim.as_bytes() {
      if emitted_pairs < pairs {
        if !inside {
          result.push_str(open);
          inside = true;
        } else {
          result.push_str(close);
          inside = false;
          emitted_pairs += 1;
        }
      }
      // Else: dangling delim past the last completed pair — drop it.
      i += dlen;
    } else {
      let next = input[i..].chars().next().map_or(0, |c| c.len_utf8());
      result.push_str(&input[i..i + next]);
      i += next;
    }
  }
  if i < bytes.len() {
    result.push_str(&input[i..]);
  }
  result
}

/// Replace `[label](https://...)` with safe `<a>` tags. Only `http(s)`
/// schemes are accepted — everything else is rendered as-is to avoid
/// `javascript:` exploits.
///
/// Accepts optional whitespace between `]` and `(` per the Markdown
/// spec subset (e.g. `[text] (url)`), but trims the URL to avoid
/// trailing whitespace being included.
pub(super) fn replace_links(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  let chars: Vec<char> = input.chars().collect();
  let len = chars.len();
  let mut i = 0;
  while i < len {
    if chars[i] == '[' {
      // Find the closing ']'.
      if let Some(close_br_rel) = chars[i + 1..].iter().position(|&c| c == ']') {
        let close_br = i + 1 + close_br_rel; // absolute index of ']'
        // Skip optional whitespace between ] and (.
        let mut maybe_open = close_br + 1;
        while maybe_open < len && chars[maybe_open].is_whitespace() {
          maybe_open += 1;
        }
        if maybe_open < len && chars[maybe_open] == '(' {
          // Find the closing ')'.
          let paren_start = maybe_open + 1;
          if let Some(close_pa_rel) = chars[paren_start..].iter().position(|&c| c == ')') {
            let close_pa = paren_start + close_pa_rel; // absolute index of ')'
            let label: String = chars[i + 1..close_br].iter().collect();
            let escaped_label = escape_html(&label);
            let url: String = chars[paren_start..close_pa].iter().collect();
            let trimmed_url = url.trim();
            if trimmed_url.starts_with("https://") || trimmed_url.starts_with("http://") {
              out.push_str(&format!(
                "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                trimmed_url, escaped_label
              ));
              i = close_pa + 1;
              continue;
            }
          }
        }
      }
    }
    out.push(chars[i]);
    i += 1;
  }
  out
}
