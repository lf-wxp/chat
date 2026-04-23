//! Scrollable message list.
//!
//! Responsible for:
//!
//! * Rendering the reactive `Vec<ChatMessage>` for the active conv.
//! * Auto-scrolling to the bottom on new outgoing / incoming messages
//!   when the viewport is already near the bottom (Req 4.10.x).
//! * Displaying a "new messages" divider at the last-seen boundary.
//! * Showing a floating "back to latest" chip when the user has
//!   scrolled up far enough for new messages to accumulate off-screen.
//! * Exposing a `scroll_to_message` imperative helper via callback.

use crate::chat::use_chat_manager;
use crate::chat_view::message_bubble::{BubbleCallbacks, MessageBubble};
use crate::i18n;
use crate::state::ConversationId;
use leptos::ev::Event;
use leptos::html;
use leptos::prelude::*;
use leptos_i18n::t_string;
use message::MessageId;
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::HtmlElement;

/// Pixel distance from the bottom that still counts as "near the
/// bottom" for the auto-scroll decision.
const NEAR_BOTTOM_PX: f64 = 80.0;

/// Properties for the scroll-to-message helper so the parent can
/// expose a stable reference for reply-jump.
#[derive(Clone, Copy)]
pub struct ScrollController {
  /// Imperative handle: jump the list to the given message id and
  /// briefly highlight it.
  pub scroll_to: Callback<MessageId>,
}

/// Scrollable message list for the active conversation.
#[component]
pub fn MessageList(
  conv: Signal<Option<ConversationId>>,
  cbs: BubbleCallbacks,
  /// Expose the imperative scroll controller back to the parent so
  /// reply jumps anywhere in the tree can reach us.
  set_controller: WriteSignal<Option<ScrollController>>,
) -> impl IntoView {
  let manager = use_chat_manager();
  let i18n = i18n::use_i18n();

  let scroll_ref = NodeRef::<html::Div>::new();

  // Observed distance from the bottom (updated on every scroll event).
  let near_bottom = RwSignal::new(true);
  // Count of messages that arrived while the user was NOT near the
  // bottom, used for the floating "new messages" badge.
  let off_screen_new = RwSignal::new(0u32);

  // Memoise the message list so we only re-render on real changes.
  let manager_for_messages = manager.clone();
  let messages = Memo::new(move |_| match conv.get() {
    Some(id) => manager_for_messages.conversation_state(&id).messages.get(),
    None => Vec::new(),
  });

  // Track the last-seen boundary so we can render the divider without
  // shifting when new messages arrive (Req 4.10.x).
  let unread_anchor = Memo::new(move |_| {
    let id = conv.get()?;
    manager.conversation_state(&id).last_seen.get_untracked()
  });

  // Auto-scroll + unread bookkeeping effect.
  {
    let messages_for_effect = messages;
    Effect::new(move |prev_len: Option<usize>| {
      let list = messages_for_effect.get();
      let prev = prev_len.unwrap_or(0);
      let curr = list.len();
      if curr > prev {
        // Appended messages → check auto-scroll behavior.
        if near_bottom.get_untracked() {
          scroll_to_bottom(&scroll_ref);
        } else {
          off_screen_new.update(|n| *n = n.saturating_add((curr - prev) as u32));
        }
      }
      curr
    });
  }

  // When the user switches conversations, jump to the bottom and reset
  // the off-screen counter.
  {
    let conv_for_effect = conv;
    Effect::new(move |_| {
      let _ = conv_for_effect.get();
      near_bottom.set(true);
      off_screen_new.set(0);
      // Defer the scroll by one tick so the list has rendered.
      request_animation_frame_scroll(scroll_ref);
    });
  }

  // Scroll handler: keep `near_bottom` in sync with actual viewport.
  let on_scroll = move |_: Event| {
    if let Some(el) = scroll_ref.get() {
      let scroll_top = el.scroll_top() as f64;
      let client_height = el.client_height() as f64;
      let scroll_height = el.scroll_height() as f64;
      let distance = scroll_height - (scroll_top + client_height);
      let near = distance <= NEAR_BOTTOM_PX;
      near_bottom.set(near);
      if near {
        off_screen_new.set(0);
      }
    }
  };

  // Imperative scroll-to-message.
  {
    let scroll_to = Callback::new(move |id: MessageId| {
      scroll_to_message(&scroll_ref, id);
    });
    set_controller.set(Some(ScrollController { scroll_to }));
  }

  // Back-to-latest click.
  let back_to_latest = move |_| {
    scroll_to_bottom(&scroll_ref);
    near_bottom.set(true);
    off_screen_new.set(0);
  };

  view! {
    <div
      node_ref=scroll_ref
      class="chat-view-scroll message-list"
      on:scroll=on_scroll
      data-testid="message-list"
    >
      {move || {
        let list = messages.get();
        if list.is_empty() {
          return view! {
            <div class="chat-view-empty">{t_string!(i18n, chat.no_messages)}</div>
          }
          .into_any();
        }
        let divider_anchor = unread_anchor.get();
        list
          .into_iter()
          .enumerate()
          .map(|(idx, msg)| {
            let show_divider = divider_anchor.map(|last| msg.id == last).unwrap_or(false)
              && idx > 0;
            view! {
              <Show when=move || show_divider fallback=|| ()>
                <div class="message-unread-divider" aria-hidden="true">
                  <span class="message-unread-divider-line"></span>
                  <span class="message-unread-divider-label">
                    {t_string!(i18n, chat.new_messages_divider)}
                  </span>
                  <span class="message-unread-divider-line"></span>
                </div>
              </Show>
              <MessageBubble msg=msg.clone() cbs=cbs />
            }
          })
          .collect_view()
          .into_any()
      }}

      // Floating "new messages" badge when scrolled up.
      <Show when=move || { off_screen_new.get() > 0 } fallback=|| ()>
        <button
          type="button"
          class="new-messages-badge"
          on:click=back_to_latest
          data-testid="new-messages-badge"
        >
          {move || format!("{} {}", off_screen_new.get(), t_string!(i18n, chat.new_messages_badge))}
        </button>
      </Show>

      <Show
        when=move || !near_bottom.get() && off_screen_new.get() == 0
        fallback=|| ()
      >
        <button
          type="button"
          class="back-to-latest"
          on:click=back_to_latest
          data-testid="back-to-latest"
        >
          {t_string!(i18n, chat.back_to_latest)}
        </button>
      </Show>
    </div>
  }
}

fn scroll_to_bottom(node_ref: &NodeRef<html::Div>) {
  if let Some(el) = node_ref.get() {
    let height = el.scroll_height();
    el.set_scroll_top(height);
  }
}

fn request_animation_frame_scroll(node_ref: NodeRef<html::Div>) {
  // Use rAF so the scroll runs after the new children render.
  let Some(window) = web_sys::window() else {
    return;
  };
  let cb = Closure::once_into_js(move || {
    scroll_to_bottom(&node_ref);
  });
  let _ = window.request_animation_frame(cb.unchecked_ref::<js_sys::Function>());
}

fn scroll_to_message(node_ref: &NodeRef<html::Div>, id: MessageId) {
  let Some(el) = node_ref.get() else { return };
  let selector = format!("[data-message-id=\"{id}\"]");
  let Ok(Some(found)) = el.query_selector(&selector) else {
    return;
  };
  let Ok(target) = found.dyn_into::<HtmlElement>() else {
    return;
  };
  // Smooth scroll the target into the middle of the viewport.
  let options = web_sys::ScrollIntoViewOptions::new();
  options.set_behavior(web_sys::ScrollBehavior::Smooth);
  options.set_block(web_sys::ScrollLogicalPosition::Center);
  target.scroll_into_view_with_scroll_into_view_options(&options);
  // Attach the highlight flash class and remove after 1.5s.
  let _ = target.class_list().add_1("message-highlight");
  let target_for_cleanup = target.clone();
  let _ = crate::utils::set_timeout_once(1_500, move || {
    let _ = target_for_cleanup
      .class_list()
      .remove_1("message-highlight");
  });
}
