//! Sticker / Emoji panel component

use leptos::prelude::*;

/// Sticker / Emoji panel
#[component]
pub fn StickerPanel(
  /// Whether to show the sticker panel
  show_sticker_panel: RwSignal<bool>,
  /// Current tab index (0=Emoji, 1=Sticker)
  sticker_tab: RwSignal<u8>,
  /// Callback to send a sticker
  on_send_sticker: Callback<(String, String)>,
  /// Callback to insert an emoji into input
  on_insert_emoji: Callback<String>,
) -> impl IntoView {
  move || {
    if !show_sticker_panel.get() {
      return view! { <div class="sticker-panel-hidden"></div> }.into_any();
    }
    let current_tab = sticker_tab.get();
    view! {
      <div class="sticker-panel">
        <div class="sticker-panel-tabs">
          <button
            class=move || if current_tab == 0 { "sticker-tab active" } else { "sticker-tab" }
            on:click=move |_| sticker_tab.set(0)
          >"😊 Emoji"</button>
          <button
            class=move || if current_tab == 1 { "sticker-tab active" } else { "sticker-tab" }
            on:click=move |_| sticker_tab.set(1)
          >"🎨 Sticker"</button>
          <button
            class="sticker-panel-close"
            on:click=move |_| show_sticker_panel.set(false)
          >"✕"</button>
        </div>
        <div class="sticker-panel-body">
          {if current_tab == 0 {
            // Emoji grid
            let emojis = vec![
              "😀","😂","🥹","😍","🤩","😘","😜","🤔","😎","🥳",
              "😢","😭","😤","🤯","😱","🫡","🤗","🫠","😴","🤮",
              "👍","👎","👏","🙌","🤝","✌️","🤞","💪","🫶","❤️",
              "🔥","⭐","🎉","🎊","💯","✅","❌","⚡","💡","🚀",
              "🌈","☀️","🌙","🍕","🍔","☕","🍺","🎵","📸","💻",
              "🐶","🐱","🐼","🦊","🐸","🐵","🦄","🐝","🦋","🌸",
            ];
            view! {
              <div class="emoji-grid">
                {emojis.into_iter().map(|e| {
                  let emoji = e.to_string();
                  let emoji_click = emoji.clone();
                  view! {
                    <button
                      class="emoji-item"
                      tabindex=0
                      on:click=move |_| on_insert_emoji.run(emoji_click.clone())
                    >{emoji}</button>
                  }
                }).collect_view()}
              </div>
            }.into_any()
          } else {
            // SVG Sticker grid — loaded from the sticker registry
            let pack = &crate::sticker::DEFAULT_PACK;
            view! {
              <div class="sticker-grid">
                {pack.stickers.iter().map(|entry| {
                  let pack_id = pack.id.to_string();
                  let sticker_id = entry.id.to_string();
                  let label = entry.label;
                  let url = format!("/stickers/{}/{}.svg", pack.id, entry.id);
                  view! {
                    <button
                      class="sticker-item"
                      tabindex=0
                      title=label
                      aria-label=label
                      on:click=move |_| on_send_sticker.run((pack_id.clone(), sticker_id.clone()))
                    >
                      <img src=url.clone() alt=label class="sticker-svg-img" loading="lazy" />
                    </button>
                  }
                }).collect_view()}
              </div>
            }.into_any()
          }}
        </div>
      </div>
    }.into_any()
  }
}
