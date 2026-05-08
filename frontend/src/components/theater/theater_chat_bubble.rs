//! Single theater chat bubble (Req 12.6).
//!
//! Stateless presentational component — reads everything from its
//! immutable [`TheaterChatMessage`] prop. Split out of the panel so
//! it can be unit-tested in isolation and so the panel file stays
//! within a reasonable size.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};

use crate::i18n;
use crate::theater::{RelativeTimeLabel, TheaterChatMessage, relative_time_label};

/// Props are passed by value (the bubble outlives its source vector
/// slot) so it can be `Clone`d cheaply by Leptos' `<For/>` machinery.
#[component]
pub fn TheaterChatBubble(
  /// Chat entry to render.
  message: TheaterChatMessage,
  /// Current wall-clock time (ms). Driven by an ambient signal in the
  /// panel so every bubble updates its relative-time label in sync.
  #[prop(into)]
  now_ms: Signal<u64>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let is_self = message.is_self;

  // Clone peer name once; resolve the localised "You" label inside a
  // reactive closure so locale changes refresh the bubble (Leptos
  // rejects naked `t_string!` calls in non-reactive component bodies).
  let peer_name = message.sender_name.clone();
  let sender_name = move || {
    if is_self {
      t_string!(i18n, theater.you_label).to_string()
    } else {
      peer_name.clone()
    }
  };

  let content = message.content.clone();
  let sent_at_ms = message.sent_at_ms;

  let time_label = move || match relative_time_label(sent_at_ms, now_ms.get()) {
    RelativeTimeLabel::JustNow => t_string!(i18n, theater.chat_time_just_now).to_string(),
    RelativeTimeLabel::MinutesAgo(minutes) => t_string!(i18n, theater.chat_time_minutes_ago)
      .to_string()
      .replace("{count}", &minutes.to_string()),
    RelativeTimeLabel::HoursAgo(hours) => t_string!(i18n, theater.chat_time_hours_ago)
      .to_string()
      .replace("{count}", &hours.to_string()),
    RelativeTimeLabel::Absolute { hours, minutes } => format!("{hours:02}:{minutes:02}"),
  };

  let bubble_class = if is_self {
    "theater-chat-bubble theater-chat-bubble--self"
  } else {
    "theater-chat-bubble theater-chat-bubble--peer"
  };

  // `aria-label` is itself reactive — we reuse the same closure but
  // through a fresh binding so `view!` can own its copy.
  let aria_name = sender_name.clone();

  view! {
    <li
      class=bubble_class
      role="listitem"
      aria-label=aria_name
      data-testid="theater-chat-bubble"
    >
      <header class="theater-chat-bubble__meta">
        <span class="theater-chat-bubble__sender">{sender_name}</span>
        <span class="theater-chat-bubble__time">{time_label}</span>
      </header>
      <p class="theater-chat-bubble__body">{content}</p>
      <span class="sr-only">{t!(i18n, theater.chat_title)}</span>
    </li>
  }
}
