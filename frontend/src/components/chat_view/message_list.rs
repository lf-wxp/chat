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
//! * Virtual scrolling when >100 messages are loaded (Task 17).
//! * Infinite scroll: loads older history from IndexedDB when the user
//!   scrolls to the top (Task 17).

use crate::chat::use_chat_manager;
use crate::components::chat_view::message_bubble::{BubbleCallbacks, MessageBubble};
use crate::components::chat_view::virtual_scroll::{
  LoadingSkeleton, VIRTUAL_THRESHOLD, VirtualMessageWindow, VirtualScrollState,
};
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

/// Pixel distance from the top that triggers an infinite-scroll load.
const NEAR_TOP_PX: f64 = 120.0;

/// Number of messages loaded per infinite-scroll page.
const LOAD_BEFORE_PAGE: usize = 50;

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

  // Virtual scroll state (height cache + loading flags).
  let vs = VirtualScrollState::new();

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
  let manager_for_anchor = manager.clone();
  let unread_anchor = Memo::new(move |_| {
    let id = conv.get()?;
    manager_for_anchor
      .conversation_state(&id)
      .last_seen
      .get_untracked()
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

  // When the user switches conversations, jump to the bottom, reset
  // the off-screen counter and the virtual-scroll state.
  {
    let conv_for_effect = conv;
    let vs_for_reset = vs.clone();
    Effect::new(move |_| {
      let _ = conv_for_effect.get();
      near_bottom.set(true);
      off_screen_new.set(0);
      vs_for_reset.reset();
      // Defer the scroll by one tick so the list has rendered.
      request_animation_frame_scroll(scroll_ref);
    });
  }

  // Scroll handler: keep `near_bottom` in sync with actual viewport
  // AND trigger infinite-scroll when near the top.
  let vs_for_scroll = vs.clone();
  let manager_for_scroll = manager.clone();
  let on_scroll = move |_: Event| {
    let Some(el) = scroll_ref.get() else { return };
    let scroll_top_val = el.scroll_top() as f64;
    let client_height = el.client_height() as f64;
    let scroll_height = el.scroll_height() as f64;

    // Drive virtual-scroll signals from the single scroll handler
    // (deduplicates the native listener previously in VirtualMessageWindow).
    vs_for_scroll.scroll_top.set(scroll_top_val);
    vs_for_scroll.viewport_h.set(client_height);

    let distance_bottom = scroll_height - (scroll_top_val + client_height);
    let near = distance_bottom <= NEAR_BOTTOM_PX;
    near_bottom.set(near);
    if near {
      off_screen_new.set(0);
    }

    // Infinite scroll: load older messages when near the top.
    if scroll_top_val <= NEAR_TOP_PX
      && !vs_for_scroll.loading_older.get_untracked()
      && vs_for_scroll.has_more.get_untracked()
      && let Some(conv_id) = conv.get_untracked()
    {
      let current_msgs = messages.get_untracked();
      if let Some(oldest) = current_msgs.first() {
        let before_ts = oldest.timestamp_ms;
        vs_for_scroll.loading_older.set(true);
        let vs_inner = vs_for_scroll.clone();
        let scroll_ref_inner = scroll_ref;

        // Capture current scroll height before prepend.
        let old_height = el.scroll_height() as f64;

        manager_for_scroll.load_older(conv_id, before_ts, LOAD_BEFORE_PAGE, move |count| {
          if count == 0 {
            vs_inner.has_more.set(false);
          } else {
            // Preserve scroll position after prepend by adjusting
            // scrollTop by the delta in scrollHeight. Deferred to
            // the next animation frame so the DOM has updated.
            let cb = Closure::once_into_js(move || {
              if let Some(el2) = scroll_ref_inner.get() {
                let new_height = el2.scroll_height() as f64;
                let delta = new_height - old_height;
                el2.set_scroll_top((el2.scroll_top() as f64 + delta) as i32);
              }
            });
            if let Some(w) = web_sys::window() {
              let _ = w.request_animation_frame(cb.unchecked_ref::<js_sys::Function>());
            }
          }
          vs_inner.loading_older.set(false);
        });
      }
    }
  };

  // Imperative scroll-to-message with IndexedDB fallback (Req 14.11.4).
  {
    let manager_for_scroll = manager.clone();
    let conv_for_scroll = conv;
    let scroll_to = Callback::new(move |id: MessageId| {
      if try_scroll_to_message(&scroll_ref, id) {
        return;
      }
      // Target not in DOM — check if it's in the loaded message list.
      let in_list = messages.with_untracked(|list| list.iter().any(|m| m.id == id));
      if in_list {
        // Message exists in list but may be outside virtual window;
        // force a re-render by nudging scrollTop so the virtual
        // window recalculates and materialises the row.
        if let Some(el) = scroll_ref.get() {
          let current = el.scroll_top() as f64;
          el.set_scroll_top((current + 1.0) as i32);
          el.set_scroll_top(current as i32);
        }
        return;
      }
      // Message not loaded at all — fetch from IndexedDB.
      if let Some(conv_id) = conv_for_scroll.get_untracked() {
        let scroll_ref_inner = scroll_ref;
        manager_for_scroll.load_jump_window(conv_id, id, move || {
          // After merge, try scrolling again on the next frame.
          let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
            try_scroll_to_message(&scroll_ref_inner, id);
          });
          if let Some(w) = web_sys::window() {
            let _ = w.request_animation_frame(cb.unchecked_ref::<js_sys::Function>());
          }
        });
      }
    });
    set_controller.set(Some(ScrollController { scroll_to }));
  }

  // Back-to-latest click.
  let back_to_latest = move |_| {
    scroll_to_bottom(&scroll_ref);
    near_bottom.set(true);
    off_screen_new.set(0);
  };

  let vs_for_view = vs.clone();

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
          view! {
            <div class="chat-view-empty">{t_string!(i18n, chat.no_messages)}</div>
          }
          .into_any()
        } else if list.len() > VIRTUAL_THRESHOLD {
          view! {
            <VirtualMessageWindow
              messages=messages.into()
              cbs=cbs
              vs=vs_for_view.clone()
              unread_anchor=unread_anchor
            />
          }
          .into_any()
        } else {
          let divider_anchor = unread_anchor.get();
          let show_loading = vs_for_view.loading_older.get();
          let no_more = !vs_for_view.has_more.get();
          view! {
            <Show when=move || show_loading fallback=|| ()>
              <LoadingSkeleton />
              <LoadingSkeleton />
              <LoadingSkeleton />
            </Show>
            // Req 14.11.3.3: "Beginning of conversation" divider when all
            // history has been loaded (has_more = false).
            <Show when=move || no_more fallback=|| ()>
              <div class="message-beginning-divider" aria-hidden="true">
                <span class="message-beginning-divider-line"></span>
                <span class="message-beginning-divider-label">
                  {t_string!(i18n, chat.beginning_of_conversation)}
                </span>
                <span class="message-beginning-divider-line"></span>
              </div>
            </Show>
            {list
              .into_iter()
              .enumerate()
              .map(|(idx, msg)| {
                let show_divider = divider_anchor
                  .map(|last| msg.id == last)
                  .unwrap_or(false)
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
              .collect_view()}
          }
          .into_any()
        }
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

/// Try to scroll to a message in the DOM. Returns `true` if the
/// element was found and scrolled into view.
fn try_scroll_to_message(node_ref: &NodeRef<html::Div>, id: MessageId) -> bool {
  let Some(el) = node_ref.get() else {
    return false;
  };
  let selector = format!("[data-message-id=\"{id}\"]");
  let Ok(Some(found)) = el.query_selector(&selector) else {
    return false;
  };
  let Ok(target) = found.dyn_into::<HtmlElement>() else {
    return false;
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
  true
}
