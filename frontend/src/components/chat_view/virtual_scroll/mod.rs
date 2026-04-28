//! Virtual scrolling for large message lists (Task 17).
//!
//! When the message count exceeds [`VIRTUAL_THRESHOLD`] the component
//! switches from a simple `collect_view()` to a windowed renderer that
//! only materialises the visible rows plus an overscan buffer.
//!
//! # Height estimation
//!
//! Because message bubbles have variable height (text wraps, images
//! have aspect ratios, voice clips have a fixed bar height) we use a
//! per-content-type *estimated* height that is refined after first
//! render via `ResizeObserver`. The resolved heights are cached in a
//! `HashMap<MessageId, f64>` so subsequent re-renders are pixel-perfect.
//!
//! # Infinite scroll (load_before)
//!
//! When the user scrolls to the top sentinel the component fires a
//! callback to load an older page from IndexedDB. The newly prepended
//! messages shift the viewport by exactly the sum of their estimated
//! heights so the reading position is preserved.

use crate::chat::models::{ChatMessage, MessageContent};
use crate::components::chat_view::message_bubble::BubbleCallbacks;
use leptos::prelude::*;
use message::MessageId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Messages below this threshold are rendered with plain
/// `collect_view()` — the virtual-scroll machinery is overkill.
pub const VIRTUAL_THRESHOLD: usize = 100;

/// Number of rows rendered above/below the visible viewport
/// (Req 14.11.2: overscan buffer of 3 items above and 3 below).
const OVERSCAN: usize = 3;

/// Maximum number of entries in the height cache. When exceeded, the
/// oldest entries are evicted (simple LRU approximation by clearing
/// half the cache). This prevents unbounded memory growth when loading
/// extensive history (P5 fix).
const HEIGHT_CACHE_MAX_SIZE: usize = 2000;

/// Base height (px) for a single-line text bubble (padding + avatar + name).
const TEXT_BUBBLE_BASE_H: f64 = 48.0;

/// Additional height (px) per wrapped line of text beyond the first.
const TEXT_BUBBLE_LINE_H: f64 = 20.0;

/// Base height (px) for a forwarded-message bubble (includes the
/// "Forwarded from" label + one line of content).
const FORWARDED_BUBBLE_BASE_H: f64 = 72.0;

/// Approximate number of characters per line of text in a bubble.
/// Lower for CJK-heavy content (vs ~80 for pure ASCII).
const CHARS_PER_LINE: usize = 40;

/// Maximum thumbnail width (px) used for image aspect-ratio calculation.
const IMAGE_THUMB_MAX_W: f64 = 300.0;

/// Minimum image bubble height (px), clamping aspect-ratio output.
const IMAGE_BUBBLE_MIN_H: f64 = 60.0;

/// Maximum image bubble height (px), clamping aspect-ratio output.
const IMAGE_BUBBLE_MAX_H: f64 = 400.0;

/// Extra vertical padding (px) added to image bubbles for the
/// timestamp / status line.
const IMAGE_BUBBLE_PADDING: f64 = 32.0;

/// Height (px) for a sticker bubble.
const STICKER_BUBBLE_H: f64 = 120.0;

/// Height (px) for a voice-clip bubble.
const VOICE_BUBBLE_H: f64 = 60.0;

/// Height (px) for a revoked-message placeholder.
const REVOKED_BUBBLE_H: f64 = 48.0;

/// Height (px) for a file-attachment bubble (filename + size line +
/// progress bar + download button). Aligns with the `.message-file`
/// card defined in `chat-messages.css`.
const FILE_BUBBLE_H: f64 = 96.0;

/// Estimated row height (px) per content type.
///
/// Height values aligned with requirements (task-item.md):
/// - Text: variable (48 px base + 20 px per wrapped line)
/// - Image: aspect-ratio proportional
/// - Voice: ~60 px
/// - Sticker: ~120 px
/// - File: ~80 px (added when Task 19 lands)
pub fn estimate_height(msg: &ChatMessage) -> f64 {
  match &msg.content {
    MessageContent::Text(t) => {
      let char_count = t.chars().count();
      let lines = (char_count as f64 / CHARS_PER_LINE as f64).ceil().max(1.0);
      TEXT_BUBBLE_BASE_H + (lines - 1.0) * TEXT_BUBBLE_LINE_H
    }
    MessageContent::Image(img) => {
      let aspect = img.height as f64 / (img.width as f64).max(1.0);
      (IMAGE_THUMB_MAX_W * aspect).clamp(IMAGE_BUBBLE_MIN_H, IMAGE_BUBBLE_MAX_H)
        + IMAGE_BUBBLE_PADDING
    }
    MessageContent::Sticker(_) => STICKER_BUBBLE_H,
    MessageContent::Voice(_) => VOICE_BUBBLE_H,
    MessageContent::File(_) => FILE_BUBBLE_H,
    MessageContent::Forwarded { content, .. } => {
      let char_count = content.chars().count();
      let lines = (char_count as f64 / CHARS_PER_LINE as f64).ceil().max(1.0);
      FORWARDED_BUBBLE_BASE_H + (lines - 1.0) * TEXT_BUBBLE_LINE_H
    }
    MessageContent::Revoked => REVOKED_BUBBLE_H,
  }
}

/// Resolved heights cache shared across renders.
///
/// Uses an approximate LRU strategy: each entry carries a monotonic
/// access counter. When the cache exceeds [`HEIGHT_CACHE_MAX_SIZE`],
/// entries with the lowest access counts are evicted first. This
/// avoids the previous issue where recent entries could be evicted
/// simply because they happened to sit near the front of the
/// HashMap iteration order (BUG-3 fix).
#[derive(Clone, Default)]
pub struct HeightCache {
  map: Rc<RefCell<HashMap<MessageId, (f64, u64)>>>,
  access_counter: Rc<RefCell<u64>>,
}

crate::wasm_send_sync!(HeightCache);

impl HeightCache {
  /// Insert a measured height. Automatically evicts older entries when
  /// the cache exceeds [`HEIGHT_CACHE_MAX_SIZE`] using LRU ordering.
  pub fn insert(&self, id: MessageId, h: f64) {
    let mut counter = self.access_counter.borrow_mut();
    *counter += 1;
    let tick = *counter;
    drop(counter);

    let mut map = self.map.borrow_mut();
    map.insert(id, (h, tick));

    if map.len() > HEIGHT_CACHE_MAX_SIZE {
      let to_remove = map.len() / 2;
      // Collect entries sorted by access time (ascending = oldest first).
      let mut entries: Vec<_> = map.iter().map(|(k, (_, tick))| (*k, *tick)).collect();
      entries.sort_by_key(|(_, tick)| *tick);
      for (key, _) in entries.into_iter().take(to_remove) {
        map.remove(&key);
      }
    }
  }

  /// Look up a cached height. Touches the access counter so the entry
  /// is considered recently used (LRU semantics).
  pub fn get(&self, id: &MessageId) -> Option<f64> {
    let mut map = self.map.borrow_mut();
    let mut counter = self.access_counter.borrow_mut();
    let entry = map.get_mut(id)?;
    *counter += 1;
    entry.1 = *counter;
    Some(entry.0)
  }

  /// Clear all cached heights.
  pub fn clear(&self) {
    self.map.borrow_mut().clear();
    *self.access_counter.borrow_mut() = 0;
  }

  /// Return the number of cached entries.
  #[cfg(test)]
  pub fn len(&self) -> usize {
    self.map.borrow().len()
  }

  /// Return `true` if the cache contains no entries.
  #[cfg(test)]
  pub fn is_empty(&self) -> bool {
    self.map.borrow().is_empty()
  }
}

/// Create a new empty height cache.
#[must_use]
pub fn new_height_cache() -> HeightCache {
  HeightCache::default()
}

/// Look up a cached height or fall back to the content-type estimate.
fn row_height(cache: &HeightCache, msg: &ChatMessage) -> f64 {
  cache.get(&msg.id).unwrap_or_else(|| estimate_height(msg))
}

/// Compute prefix sums of row heights for binary-search lookups.
///
/// `prefix[i]` is the total height of rows `0..i` (exclusive), so
/// `prefix[0] == 0.0` and `prefix[n]` equals the total height.
fn prefix_heights(messages: &[ChatMessage], cache: &HeightCache) -> Vec<f64> {
  let mut prefix = Vec::with_capacity(messages.len() + 1);
  prefix.push(0.0);
  let mut acc = 0.0;
  for m in messages {
    acc += row_height(cache, m);
    prefix.push(acc);
  }
  prefix
}

/// Compute the visible window given the full message list and the
/// current `scrollTop` / viewport `height`.
///
/// Uses binary search on prefix height sums for O(log n) window
/// start lookup instead of O(n) linear scan (OPT-5 fix).
///
/// Returns `(start_index, end_index, offset_y)` where `offset_y` is
/// the CSS `translateY` for the first rendered row.
pub fn compute_window(
  messages: &[ChatMessage],
  cache: &HeightCache,
  scroll_top: f64,
  viewport_h: f64,
) -> (usize, usize, f64) {
  if messages.is_empty() {
    return (0, 0, 0.0);
  }

  let prefix = prefix_heights(messages, cache);

  // Binary search: find the first row whose bottom edge is past
  // scroll_top. Equivalent to finding the smallest i such that
  // prefix[i+1] > scroll_top.
  let start = match prefix[1..].binary_search_by(|probe| {
    probe
      .partial_cmp(&scroll_top)
      .unwrap_or(std::cmp::Ordering::Equal)
  }) {
    // Exact match: scroll_top lands exactly on a row boundary.
    Ok(i) => i + 1,
    // No exact match: i is where scroll_top would be inserted.
    Err(i) => i,
  };

  // Walk forward from start until we've filled the viewport.
  let mut filled = 0.0;
  let mut end = start;
  for m in &messages[start..] {
    end += 1;
    filled += row_height(cache, m);
    if filled >= viewport_h {
      break;
    }
  }

  // Apply overscan.
  let start = start.saturating_sub(OVERSCAN);
  let end = (end + OVERSCAN).min(messages.len());

  // offset_y is the prefix sum up to `start`.
  let y = prefix[start];

  (start, end, y)
}

/// Compute the total content height of all messages.
pub fn total_height(messages: &[ChatMessage], cache: &HeightCache) -> f64 {
  messages.iter().map(|m| row_height(cache, m)).sum()
}

/// Virtual scroll state provided to the message list.
#[derive(Clone)]
pub struct VirtualScrollState {
  /// Cached measured heights.
  pub cache: HeightCache,
  /// Whether we are currently loading an older page (prevents
  /// duplicate requests while the fetch is in flight).
  pub loading_older: RwSignal<bool>,
  /// Whether there are more messages to load (set to `false` when
  /// `load_before` returns an empty page).
  pub has_more: RwSignal<bool>,
  /// Current scrollTop of the viewport (driven by the parent
  /// `on:scroll` handler).
  pub scroll_top: RwSignal<f64>,
  /// Current viewport height in pixels.
  pub viewport_h: RwSignal<f64>,
}

impl Default for VirtualScrollState {
  fn default() -> Self {
    Self::new()
  }
}

impl VirtualScrollState {
  /// Create a fresh virtual scroll state.
  #[must_use]
  pub fn new() -> Self {
    Self {
      cache: new_height_cache(),
      loading_older: RwSignal::new(false),
      has_more: RwSignal::new(true),
      scroll_top: RwSignal::new(0.0),
      viewport_h: RwSignal::new(600.0),
    }
  }

  /// Reset the state (e.g. when switching conversations).
  pub fn reset(&self) {
    self.cache.clear();
    self.loading_older.set(false);
    self.has_more.set(true);
    self.scroll_top.set(0.0);
    self.viewport_h.set(600.0);
  }

  /// Record a measured height for a message.
  pub fn set_height(&self, id: MessageId, height: f64) {
    self.cache.insert(id, height);
  }
}

crate::wasm_send_sync!(VirtualScrollState);

/// Properties for the skeleton placeholder shown while loading older
/// messages.
#[component]
pub fn LoadingSkeleton() -> impl IntoView {
  view! {
    <div class="message-skeleton" aria-hidden="true" data-testid="message-skeleton">
      <div class="skeleton-avatar"></div>
      <div class="skeleton-lines">
        <div class="skeleton-line skeleton-line-long"></div>
        <div class="skeleton-line skeleton-line-short"></div>
      </div>
    </div>
  }
}

/// Render the virtual-scrolled message window.
///
/// Called from `MessageList` when the message count exceeds
/// [`VIRTUAL_THRESHOLD`]. The outer `<div>` carries a spacer element
/// whose height equals `total_height` so the browser scrollbar tracks
/// correctly, and an inner positioned container that only renders the
/// visible window.
#[component]
pub fn VirtualMessageWindow(
  messages: Signal<Vec<ChatMessage>>,
  cbs: BubbleCallbacks,
  vs: VirtualScrollState,
  #[prop(optional)] unread_anchor: Option<Memo<Option<MessageId>>>,
) -> impl IntoView {
  use crate::components::chat_view::message_bubble::MessageBubble;
  use crate::i18n;
  use leptos_i18n::t_string;

  let i18n = i18n::use_i18n();

  // Derive the windowed slice reactively from the signals stored in
  // `vs` (updated by the parent `on:scroll` handler).
  let cache = vs.cache.clone();
  let cache2 = cache.clone();

  // N2 optimisation: total_height only depends on messages (and cache),
  // not on scrollTop/viewport.  Extract it into its own memo so that
  // scrolling does not trigger an O(n) re-computation.
  let total_height_memo = Memo::new(move |_| messages.with(|list| total_height(list, &cache)));

  let window_memo = Memo::new(move |_| {
    let st = vs.scroll_top.get();
    let vh = vs.viewport_h.get();
    messages.with(|list| {
      let (start, end, offset) = compute_window(list, &cache2, st, vh);
      let total = total_height_memo.get();
      (start, end, offset, total)
    })
  });

  view! {
    {move || {
      let (start, end, offset, total) = window_memo.get();
      let spacer_h = format!("height:{total}px;position:relative;");
      let inner_style = format!(
        "position:absolute;top:0;left:0;right:0;transform:translateY({offset}px);",
      );

      let divider_anchor = unread_anchor.and_then(|m| m.get());

      messages.with(|list| {
      // Only clone the visible window (~10-20 messages) rather than the
      // entire list (R5 + OPT-1 fix).
      let visible: Vec<_> = list[start..end].to_vec();

      view! {
        <div style=spacer_h>
          // Skeleton loader at the top while loading older messages.
          <Show when=move || vs.loading_older.get() fallback=|| ()>
            <div style="position:absolute;top:0;left:0;right:0;">
              <LoadingSkeleton />
              <LoadingSkeleton />
              <LoadingSkeleton />
            </div>
          </Show>

          // Req 14.11.3.3: "Beginning of conversation" divider when
          // all history has been loaded (has_more = false).
          // N1 fix: read `has_more` reactively inside the `when` closure
          // instead of snapshotting it with `get_untracked()` outside.
          <Show when=move || !vs.has_more.get() && start == 0 fallback=|| ()>
            <div
              class="message-beginning-divider"
              style="position:absolute;top:0;left:0;right:0;"
              aria-hidden="true"
            >
              <span class="message-beginning-divider-line"></span>
              <span class="message-beginning-divider-label">
                {t_string!(i18n, chat.beginning_of_conversation)}
              </span>
              <span class="message-beginning-divider-line"></span>
            </div>
          </Show>

          <div style=inner_style>
            {visible
              .into_iter()
              .enumerate()
              .map(|(local_idx, msg)| {
                let global_idx = start + local_idx;
                let show_divider = divider_anchor
                  .map(|last| msg.id == last)
                  .unwrap_or(false)
                  && global_idx > 0;
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
                  <MessageBubble msg=msg.clone() cbs=cbs vs=vs.clone() />
                }
              })
              .collect_view()}
          </div>
        </div>
      }})
    }}
  }
}

#[cfg(test)]
mod tests;
