//! Single row inside the room member list.
//!
//! Each row shows the member's avatar, display name, role badge
//! (👑 Owner / ⭐ Admin) and a mute indicator when applicable.
//! Clicking the row toggles the per-member context menu.

use leptos::prelude::*;
use leptos_i18n::t_string;
use message::UserId;
use message::types::{MemberInfo, RoomRole};

use crate::components::room::member_context_menu::MemberContextMenu;
use crate::components::room::utils::{MemberAction, is_currently_muted, mute_remaining_seconds};
use crate::i18n;
use crate::identicon::generate_identicon_data_uri;
use crate::state::use_app_state;

/// Split `name` into highlighted and non-highlighted fragments by
/// matching `query` case-insensitively. Operates on `char` boundaries
/// to remain safe for multi-byte UTF-8 input (e.g. characters whose
/// lowercase form has a different byte length, such as the Turkish
/// `İ` → `i\u{307}` mapping).
///
/// Returned fragments preserve the **original** casing of `name` so
/// the rendered text is unchanged; only the `bool` flag indicates
/// whether each slice should be wrapped in `<mark>`.
pub(crate) fn highlight_name(name: &str, query: &str) -> Vec<(String, bool)> {
  if query.is_empty() {
    return vec![(name.to_string(), false)];
  }

  // Normalise both strings to lowercase character sequences so the
  // comparison is independent of byte lengths.
  let name_chars: Vec<(usize, char)> = name.char_indices().collect();
  let lower_name: Vec<char> = name.chars().flat_map(char::to_lowercase).collect();
  let lower_query: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();

  if lower_query.is_empty() || lower_query.len() > lower_name.len() {
    return vec![(name.to_string(), false)];
  }

  // Build a mapping from each lower_name char index back to the
  // originating char index in `name_chars`. Because a single original
  // char may expand to multiple lowercase chars, each slot points to
  // the *original* char that produced it.
  let mut origin_for_lower: Vec<usize> = Vec::with_capacity(lower_name.len());
  for (orig_idx, ch) in name.chars().enumerate() {
    let expansion_len = ch.to_lowercase().count().max(1);
    for _ in 0..expansion_len {
      origin_for_lower.push(orig_idx);
    }
  }

  let mut fragments: Vec<(String, bool)> = Vec::new();
  let mut origin_cursor: usize = 0;
  let mut lower_cursor: usize = 0;

  while lower_cursor + lower_query.len() <= lower_name.len() {
    if lower_name[lower_cursor..lower_cursor + lower_query.len()] == lower_query[..] {
      let match_origin_start = origin_for_lower[lower_cursor];
      let match_origin_end_inclusive = origin_for_lower[lower_cursor + lower_query.len() - 1];
      let match_origin_end = match_origin_end_inclusive + 1;

      // Push the unmatched prefix.
      if match_origin_start > origin_cursor {
        let s = char_slice(&name_chars, name, origin_cursor, match_origin_start);
        if !s.is_empty() {
          fragments.push((s, false));
        }
      }
      // Push the matched span.
      let matched = char_slice(&name_chars, name, match_origin_start, match_origin_end);
      if !matched.is_empty() {
        fragments.push((matched, true));
      }
      origin_cursor = match_origin_end;
      // Advance lower_cursor past the entire matched span (including
      // any extra lowercase chars produced by the final original char).
      let mut new_lower = lower_cursor + lower_query.len();
      while new_lower < lower_name.len() && origin_for_lower[new_lower] < origin_cursor {
        new_lower += 1;
      }
      lower_cursor = new_lower;
    } else {
      lower_cursor += 1;
    }
  }

  // Tail.
  if origin_cursor < name_chars.len() {
    let tail = char_slice(&name_chars, name, origin_cursor, name_chars.len());
    if !tail.is_empty() {
      fragments.push((tail, false));
    }
  }

  if fragments.is_empty() {
    fragments.push((name.to_string(), false));
  }
  fragments
}

/// Slice the original `name` between two char indices (inclusive of
/// `start`, exclusive of `end`), returning a fresh `String`. Operates
/// on byte offsets stored in `char_indices` so it never panics on
/// multi-byte boundaries.
fn char_slice(char_indices: &[(usize, char)], name: &str, start: usize, end: usize) -> String {
  if start >= end || start >= char_indices.len() {
    return String::new();
  }
  let byte_start = char_indices[start].0;
  let byte_end = if end >= char_indices.len() {
    name.len()
  } else {
    char_indices[end].0
  };
  name[byte_start..byte_end].to_string()
}

/// A single member row.
#[component]
pub fn MemberRow(
  /// Reactive member info. The component re-renders whenever the
  /// underlying signal updates, so role / mute / nickname changes are
  /// reflected immediately (Req 15.5.39).
  #[prop(into)]
  member: Signal<MemberInfo>,
  /// Current user's room role (for gating actions).
  #[prop(into)]
  actor_role: Signal<RoomRole>,
  /// Current user's id (used for `is_self` check).
  #[prop(into)]
  actor_id: Signal<Option<UserId>>,
  /// Search query used to highlight matching substrings.
  #[prop(into)]
  query: Signal<String>,
  /// Optional username disambiguator badge text (Req 15.1.5 / 15.1.6).
  /// When `Some(..)`, the row appends a small `(badge)` suffix next to
  /// the highlighted nickname.
  #[prop(into, optional)]
  username_badge: Option<Signal<Option<String>>>,
  /// Called when a moderation action is picked from the context menu.
  on_action: Callback<(UserId, MemberAction)>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  // Global 1 Hz tick (Sprint 4.3) so the timed-mute countdown badge
  // stays fresh without spinning up a per-row interval.
  let tick = app_state.now_tick;

  let display_name = Memo::new(move |_| {
    member.with(|m| {
      if m.nickname.is_empty() {
        m.user_id.to_string()
      } else {
        m.nickname.clone()
      }
    })
  });

  let avatar = Memo::new(move |_| display_name.with(|n| generate_identicon_data_uri(n)));

  let menu_open = RwSignal::new(false);

  let is_self_signal = Memo::new(move |_| {
    let actor = actor_id.get();
    let target = member.with(|m| m.user_id.clone());
    actor.is_some_and(|id| id == target)
  });
  // Self-rows show a "Leave" action in their context menu, so the
  // button must always be clickable (BUG-NEW-01 fix — previously
  // `!is_self_signal.get()` which made Leave unreachable).
  let can_open_menu = Memo::new(move |_| true);

  let highlighted = Memo::new(move |_| highlight_name(&display_name.get(), &query.get()));

  let role_badge = move || {
    member.with(|m| match m.role {
      RoomRole::Owner => "👑",
      RoomRole::Admin => "⭐",
      RoomRole::Member => "",
    })
  };

  let role_aria = move || {
    member.with(|m| match m.role {
      RoomRole::Owner => t_string!(i18n, room.owner).to_string(),
      RoomRole::Admin => t_string!(i18n, room.admin).to_string(),
      RoomRole::Member => t_string!(i18n, room.member).to_string(),
    })
  };

  // Composite ARIA label that includes the displayed nickname, role
  // and current mute status so screen readers announce all three
  // (Req 15.6.48 / 15.6.49). Example:
  //   "alice — Admin, currently muted"
  let row_aria_label = move || {
    let name = display_name.get();
    let role = role_aria();
    let muted = member.with(is_currently_muted);
    if muted {
      format!("{name} — {role}, {}", t_string!(i18n, room.muted_indicator))
    } else {
      format!("{name} — {role}")
    }
  };

  let mute_badge = move || {
    // Subscribe to the local tick so the countdown re-evaluates every
    // second alongside the global muted-input indicator.
    let _ = tick.get();
    member.with(|m| {
      if !is_currently_muted(m) {
        return None;
      }
      let remaining = mute_remaining_seconds(m);
      let label = match remaining {
        Some(secs) => format!(
          "{} {:02}:{:02}",
          t_string!(i18n, room.muted_badge),
          secs / 60,
          secs % 60
        ),
        None => t_string!(i18n, room.muted_permanent).to_string(),
      };
      Some(label)
    })
  };

  let on_pick = Callback::new(move |action: MemberAction| {
    let target_id = member.with(|m| m.user_id.clone());
    on_action.run((target_id, action));
  });

  view! {
    <li
      class="room-member-row"
      class:room-member-row--self=move || is_self_signal.get()
      class:room-member-row--muted=move || member.with(is_currently_muted)
      data-testid="room-member-row"
    >
      <button
        type="button"
        class="room-member-row__button"
        aria-label=row_aria_label
        disabled=move || !can_open_menu.get()
        on:click=move |_| {
          if can_open_menu.get_untracked() {
            menu_open.update(|v| *v = !*v);
          }
        }
      >
        <img
          class="room-member-row__avatar"
          src=move || avatar.get()
          alt=""
          width="36"
          height="36"
        />
        <span class="room-member-row__info">
          <span class="room-member-row__name">
            {move || {
              let fragments = highlighted.get();
              fragments
                .into_iter()
                .map(|(fragment, matched)| {
                  if matched {
                    view! {
                      <mark class="room-member-row__highlight">{fragment}</mark>
                    }
                    .into_any()
                  } else {
                    view! { <span>{fragment}</span> }.into_any()
                  }
                })
                .collect_view()
            }}
            <Show when=move || !role_badge().is_empty()>
              <span class="room-member-row__badge" aria-label=role_aria>
                {role_badge}
              </span>
            </Show>
            <Show when=move || username_badge.and_then(|s| s.get()).is_some()>
              <small
                class="room-member-row__username-badge"
                aria-label=move || t_string!(i18n, room.username_aria)
                data-testid="room-member-username-badge"
              >
                {move || {
                  username_badge
                    .and_then(|s| s.get())
                    .map(|u| format!("({u})"))
                    .unwrap_or_default()
                }}
              </small>
            </Show>
          </span>
          <Show when=move || mute_badge().is_some()>
            <span class="room-member-row__mute" aria-live="polite">
              {move || mute_badge().unwrap_or_default()}
            </span>
          </Show>
        </span>
      </button>

      <Show when=move || menu_open.get()>
        <MemberContextMenu
          target=member
          actor_role=actor_role
          is_self=Signal::derive(move || is_self_signal.get())
          on_pick=on_pick
          on_close=Callback::new(move |()| menu_open.set(false))
        />
      </Show>
    </li>
  }
}
