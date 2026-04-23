//! Sticker panel overlay.
//!
//! Tabs across the top switch between sticker packs, a search box
//! filters within the current pack, and a grid of clickable tiles
//! dispatches `send_sticker`. The built-in "emoji" pack uses emoji
//! glyphs as both thumbnail and `sticker_id`, so no binary assets are
//! required for the default experience; the on-wire format is still
//! `ChatSticker { pack_id, sticker_id, ... }` so real packs can ship
//! later without breaking consumers.

use crate::chat::use_chat_manager;
use crate::i18n;
use crate::state::ConversationId;
use leptos::prelude::*;
use leptos_i18n::t_string;

/// A single sticker pack.
#[derive(Clone, Debug)]
struct Pack {
  /// Pack identifier (sent on the wire).
  id: &'static str,
  /// Tab label.
  label: &'static str,
  /// Stickers in the pack: `(sticker_id, glyph_for_preview)`. For
  /// emoji packs the glyph and id are the same.
  stickers: &'static [&'static str],
}

/// Built-in sticker packs. Mirrors what a real manifest.json would
/// expose and avoids a runtime fetch for the default experience.
const PACKS: &[Pack] = &[
  Pack {
    id: "emoji-smileys",
    label: "Smileys",
    stickers: &[
      "😀", "😃", "😄", "😁", "😆", "🥹", "😅", "😂", "🤣", "🥲", "😊", "😇", "🙂", "🙃", "😉",
      "😌", "😍", "🥰", "😘", "😗", "😙", "😚", "😋", "😛", "😝", "😜", "🤪", "🤨", "🧐", "🤓",
    ],
  },
  Pack {
    id: "emoji-animals",
    label: "Animals",
    stickers: &[
      "🐶", "🐱", "🐭", "🐹", "🐰", "🦊", "🐻", "🐼", "🐨", "🐯", "🦁", "🐮", "🐷", "🐸", "🐵",
      "🐔", "🐧", "🐦", "🐤", "🦆", "🦅", "🦉", "🦇", "🐺", "🐗", "🐴", "🦄", "🐝", "🐛", "🦋",
    ],
  },
  Pack {
    id: "emoji-gestures",
    label: "Gestures",
    stickers: &[
      "👍", "👎", "👏", "🙌", "👐", "🤲", "🤝", "🙏", "✌️", "🤞", "🤟", "🤘", "🤙", "👈", "👉",
      "👆", "👇", "☝️", "✋", "🤚", "🖐", "🖖", "👋", "🤌", "🤏", "💪", "🦾", "🦵", "🦶", "👣",
    ],
  },
];

/// Sticker panel overlay.
#[component]
pub fn StickerPanel(
  /// Active conversation (required to dispatch).
  conv: Signal<Option<ConversationId>>,
  /// Visibility signal; flipped to `false` after a successful pick.
  visible: RwSignal<bool>,
) -> impl IntoView {
  let manager = use_chat_manager();
  let i18n = i18n::use_i18n();

  let active_pack = RwSignal::new(0usize);
  let search = RwSignal::new(String::new());

  // Filter stickers by search query (case-insensitive substring match
  // on the sticker glyph). Keeps the match set small enough that a
  // quadratic filter is fine.
  let filtered = Memo::new(move |_| {
    let idx = active_pack.get();
    let query = search.get().trim().to_lowercase();
    let pack = &PACKS[idx];
    pack
      .stickers
      .iter()
      .filter(|s| query.is_empty() || s.to_lowercase().contains(&query))
      .copied()
      .collect::<Vec<_>>()
  });

  let pick_manager = StoredValue::new(manager.clone());

  view! {
    <Show when=move || visible.get() fallback=|| ()>
      <div class="sticker-panel" role="dialog" data-testid="sticker-panel">
        <div class="sticker-panel-search">
          <input
            type="search"
            placeholder=move || t_string!(i18n, chat.sticker_search)
            aria-label=move || t_string!(i18n, chat.sticker_search)
            prop:value=move || search.get()
            on:input=move |ev| {
              if let Some(target) = event_target_value_opt(&ev) {
                search.set(target);
              }
            }
          />
        </div>

        <div class="sticker-panel-tabs" role="tablist">
          {PACKS
            .iter()
            .enumerate()
            .map(|(idx, pack)| {
              let label: String = match pack.id {
                "emoji-smileys" => t_string!(i18n, chat.sticker_pack_smileys).into(),
                "emoji-animals" => t_string!(i18n, chat.sticker_pack_animals).into(),
                "emoji-gestures" => t_string!(i18n, chat.sticker_pack_gestures).into(),
                _ => pack.label.to_string(),
              };
              view! {
                <button
                  type="button"
                  class=move || {
                    if active_pack.get() == idx {
                      "sticker-panel-tab active".to_string()
                    } else {
                      "sticker-panel-tab".to_string()
                    }
                  }
                  role="tab"
                  aria-selected=move || active_pack.get() == idx
                  on:click=move |_| {
                    active_pack.set(idx);
                    search.set(String::new());
                  }
                >
                  {label}
                </button>
              }
            })
            .collect_view()}
        </div>

        <div class="sticker-panel-grid" role="tabpanel">
          {move || {
            let list = filtered.get();
            if list.is_empty() {
              return view! {
                <div class="sticker-panel-empty">
                  {t_string!(i18n, chat.sticker_no_results)}
                </div>
              }
              .into_any();
            }
            list
              .into_iter()
              .map(|glyph| {
                view! {
                  <button
                    type="button"
                    class="sticker-panel-item"
                    aria-label=glyph
                    on:click=move |_| {
                      let Some(conv_id) = conv.get_untracked() else {
                        return;
                      };
                      let pack_id = PACKS[active_pack.get_untracked()].id.to_string();
                      pick_manager.with_value(|m| {
                        let _ = m.send_sticker(conv_id, pack_id, glyph.to_string());
                      });
                      visible.set(false);
                    }
                  >
                    {glyph}
                  </button>
                }
              })
              .collect_view()
              .into_any()
          }}
        </div>
      </div>
    </Show>
  }
}

/// Extract `value` from an `input` event on an `<input>` element.
fn event_target_value_opt(ev: &leptos::ev::Event) -> Option<String> {
  use wasm_bindgen::JsCast;
  let target = ev.target()?;
  target
    .dyn_into::<web_sys::HtmlInputElement>()
    .ok()
    .map(|el| el.value())
}
