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
use icondata as i;
use leptos::ev;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;
use leptos_use::use_event_listener;
use wasm_bindgen::JsCast;
use web_sys::Element;

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

  // Dismiss the panel when the user clicks anywhere outside it. We
  // previously rendered a full-screen `.sticker-panel-backdrop` for
  // this purpose, but that overlay sat in the viewport stacking
  // context (`position: fixed`) while the panel itself was bound to
  // its `chat-input-bar` ancestor's stacking context — so on certain
  // layouts the backdrop ended up *above* the panel and intercepted
  // pointer events on the glyph buttons (manifested as Playwright
  // sticker pick failures: "backdrop intercepts pointer events").
  // Switching to a click-outside listener (the same pattern used by
  // `SidebarConversationMenu`) sidesteps the stacking-context maze
  // entirely and keeps the bottom-sheet anchored to the composer.
  let _ = use_event_listener(
    leptos_use::use_window(),
    ev::pointerdown,
    move |ev: web_sys::PointerEvent| {
      if !visible.get_untracked() {
        return;
      }
      let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
        return;
      };
      let inside_panel = target.closest(".sticker-panel").ok().flatten().is_some();
      // The chat-input-bar's smiley button toggles the panel itself;
      // letting its click reach `visible.update(|v| !*v)` would
      // re-open the panel on the same gesture that closed it. The
      // composer marks every chat-input-btn with that class.
      let on_trigger = target.closest(".chat-input-btn").ok().flatten().is_some();
      if !inside_panel && !on_trigger {
        visible.set(false);
      }
    },
  );

  // Escape closes the panel — the same affordance every other
  // dismissable popover in the app provides.
  let _ = use_event_listener(
    leptos_use::use_window(),
    ev::keydown,
    move |ev: web_sys::KeyboardEvent| {
      if !visible.get_untracked() {
        return;
      }
      if crate::utils::safe_key(&ev) == "Escape" {
        visible.set(false);
        ev.stop_propagation();
      }
    },
  );

  view! {
    <leptos::portal::Portal mount=resolve_mount()>
      <Show when=move || visible.get() fallback=|| ()>
        <div class="sticker-panel" role="dialog" data-testid="sticker-panel">
          // Header: search + close
          <div class="sticker-panel-header">
            <div class="sticker-panel-search">
              <Icon icon=i::LuSearch attr:class="sticker-panel-search-icon" />
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
            <button
              type="button"
              class="sticker-panel-close-btn"
              aria-label=move || t_string!(i18n, common.close)
              on:click=move |_| visible.set(false)
            >
              <Icon icon=i::LuX />
            </button>
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
                      data-testid="sticker-panel-item"
                      data-glyph=glyph
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
    </leptos::portal::Portal>
  }
}

/// Resolve the DOM node the sticker-panel portal mounts under.
///
/// Mirrors `ModalWrapper::resolve_mount`: prefer `#modal-root`
/// (rendered directly under `<body>` by `ModalManager`) so the panel
/// escapes any ancestor stacking context that would otherwise drop
/// it below the top-bar / chat-view chrome. Falls back to `<body>`
/// for the unauthenticated shell where `ModalManager` isn't mounted
/// yet.
fn resolve_mount() -> web_sys::Element {
  use wasm_bindgen::JsCast;
  let document = web_sys::window()
    .and_then(|w| w.document())
    .expect("window.document must be available");
  document
    .get_element_by_id("modal-root")
    .or_else(|| {
      document
        .body()
        .map(|b| b.unchecked_into::<web_sys::Element>())
    })
    .expect("document.body or #modal-root must exist before the sticker panel is rendered")
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
