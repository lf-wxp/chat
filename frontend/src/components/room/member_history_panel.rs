//! Member moderation history panel (Req 15.6.50 — Sprint 5.2).
//!
//! Renders the rolling client-side moderation log for a given room.
//! Only visible to Owners and Admins. Entries are appended by the
//! signaling message handler whenever a `ModerationNotification` is
//! received and capped at `MAX_MODERATION_LOG` per room.

use chrono::{TimeZone, Utc};
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::UserId;
use message::signaling::ModerationAction;
use message::types::RoomId;

use crate::components::room::modal_wrapper::{ModalSize, ModalWrapper};
use crate::components::room::utils::interpolate_count;
use crate::i18n;
use crate::state::use_app_state;
use icondata as i;
use leptos_icons::Icon;

/// Resolve a target `UserId` to a human-readable display name, looking
/// up the nickname from the current room's member list. Falls back to
/// `UserId::to_string()` when the member has no nickname or has left
/// the room (BUG-NEW-06 fix).
fn resolve_target_name(
  app_state: &crate::state::AppState,
  room_id: &RoomId,
  target: &UserId,
) -> String {
  app_state.room_members.with(|map| {
    map
      .get(room_id)
      .and_then(|list| list.iter().find(|m| &m.user_id == target))
      .map(|m| {
        if m.nickname.is_empty() {
          m.user_id.to_string()
        } else {
          m.nickname.clone()
        }
      })
      .unwrap_or_else(|| target.to_string())
  })
}

/// Format a nanosecond timestamp using a locale-aware date pattern
/// (BUG-NEW-08 fix). The caller resolves the format string via
/// `t_string!(i18n, room.date_format)` and passes it here; this
/// avoids depending on the `I18nContext` type in a standalone function.
fn format_timestamp(date_format: &str, nanos: i64) -> String {
  Utc.timestamp_nanos(nanos).format(date_format).to_string()
}

/// Format a duration in seconds using locale-aware unit labels
/// (BUG-NEW-08 fix). The caller resolves the three template strings
/// via `t_string!` and passes them here; this avoids depending on
/// the `I18nContext` type in a standalone function.
fn format_duration(
  secs_template: &str,
  mins_template: &str,
  hours_template: &str,
  secs: u64,
) -> String {
  if secs < 60 {
    interpolate_count(secs_template, secs as usize)
  } else if secs < 3600 {
    interpolate_count(mins_template, (secs / 60) as usize)
  } else {
    interpolate_count(hours_template, (secs / 3600) as usize)
  }
}

/// Member moderation history panel (modal).
#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn MemberHistoryPanel(
  /// Room whose moderation log should be displayed.
  #[prop(into)]
  room_id: Signal<RoomId>,
  /// Fires when the user dismisses the modal.
  on_close: Callback<()>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();

  let entries = Memo::new(move |_| {
    let rid = room_id.get();
    app_state
      .moderation_log
      .with(|map| map.get(&rid).cloned().unwrap_or_default())
  });

  view! {
    <ModalWrapper
      on_close=on_close
      size=ModalSize::Medium
      class="member-history-panel"
      labelled_by="member-history-title"
      testid="member-history-panel"
    >
      <header class="modal-header">
        <h2 id="member-history-title" class="modal-title">
          {t!(i18n, room.history_title)}
        </h2>
        <button
          type="button"
          class="modal-close"
          aria-label=move || t_string!(i18n, common.close)
          on:click=move |_| on_close.run(())
        ><Icon icon=i::LuX /></button>
      </header>
      <div class="modal-body member-history-panel__body">
        <Show
          when=move || !entries.get().is_empty()
          fallback=move || view! {
            <p class="member-history-panel__empty">{t!(i18n, room.history_empty)}</p>
          }
        >
          <ul class="member-history-panel__list" role="list">
            <For
              each=move || {
                // Newest first.
                let mut list = entries.get();
                list.reverse();
                list
              }
              key=|e| (e.timestamp_nanos, e.target.clone(), e.action as u8)
              children=move |entry| {
                let action_key = entry.action;
                // Resolve locale-aware templates once per entry.
                let date_fmt = t_string!(i18n, room.date_format);
                let secs_tpl = t_string!(i18n, room.duration_seconds);
                let mins_tpl = t_string!(i18n, room.duration_minutes);
                let hours_tpl = t_string!(i18n, room.duration_hours);
                let when = format_timestamp(date_fmt, entry.timestamp_nanos);
                let target = resolve_target_name(&app_state, &room_id.get(), &entry.target);
                let duration = entry.duration_secs.map(|s| {
                  format_duration(secs_tpl, mins_tpl, hours_tpl, s)
                });
                let action_label = move || match action_key {
                  ModerationAction::Kicked => t_string!(i18n, room.kick).to_string(),
                  ModerationAction::Muted => t_string!(i18n, room.mute).to_string(),
                  ModerationAction::Unmuted => t_string!(i18n, room.unmute).to_string(),
                  ModerationAction::Banned => t_string!(i18n, room.ban).to_string(),
                  ModerationAction::Unbanned => t_string!(i18n, room.unban).to_string(),
                  ModerationAction::Promoted => t_string!(i18n, room.promote).to_string(),
                  ModerationAction::Demoted => t_string!(i18n, room.demote).to_string(),
                };
                view! {
                  <li class="member-history-panel__entry" data-testid="member-history-entry">
                    <span class="member-history-panel__when">{when}</span>
                    <span class="member-history-panel__action">{action_label}</span>
                    <span class="member-history-panel__target">{target}</span>
                    <Show when={
                      let d = duration.clone();
                      move || d.is_some()
                    }>
                      <span class="member-history-panel__duration">
                        {duration.clone().unwrap_or_default()}
                      </span>
                    </Show>
                  </li>
                }
              }
            />
          </ul>
        </Show>
      </div>
    </ModalWrapper>
  }
}
