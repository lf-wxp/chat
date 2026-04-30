//! Room member list panel (Req 15.4).
//!
//! Responsibilities:
//!
//! * Render the currently joined members sorted by role + join time.
//! * Expose a case-insensitive search filter over nicknames and
//!   usernames.
//! * Emit moderation signalling messages when an action is picked
//!   from a member's context menu, guarded by confirmation dialogs
//!   for destructive actions.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::UserId;
use message::types::{MemberInfo, RoomId};

use crate::components::room::confirm_dialog::{ConfirmDialog, ConfirmTone};
use crate::components::room::invite_member_modal::InviteMemberModal;
use crate::components::room::member_history_panel::MemberHistoryPanel;
use crate::components::room::member_row::MemberRow;
use crate::components::room::mute_duration_picker::MuteDurationPicker;
use crate::components::room::room_settings_modal::RoomSettingsModal;
use crate::components::room::utils::{
  MemberAction, current_role, filter_members, interpolate_count, interpolate_name,
  interpolate_query, member_action_meta, member_needs_username_badge,
};
use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::signaling::use_signaling_client;
use crate::state::use_app_state;

/// Pending action awaiting user confirmation.
#[derive(Debug, Clone)]
struct PendingConfirm {
  action: MemberAction,
  target: UserId,
  target_name: String,
}

/// Member list panel for the currently active room.
#[component]
pub fn MemberListPanel(
  /// Room whose members should be rendered.
  #[prop(into)]
  room_id: Signal<RoomId>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();

  let query = RwSignal::new(String::new());
  let pending_confirm = RwSignal::new(Option::<PendingConfirm>::None);
  let pending_mute = RwSignal::new(Option::<(UserId, String)>::None);
  let settings_open = RwSignal::new(false);
  // Batch moderation state (Req 15.5.44 — Sprint 5.1).
  let batch_mode = RwSignal::new(false);
  let batch_selection = RwSignal::new(Vec::<UserId>::new());
  let batch_mute_open = RwSignal::new(false);
  /// Hard cap from Req 15.5.44 (max 5 members per batch operation).
  const BATCH_MAX: usize = 5;
  // Sprint 5.2 — Owner/Admin moderation history modal.
  let history_open = RwSignal::new(false);
  // Req 4.3 — Owner "Invite member" modal.
  let invite_open = RwSignal::new(false);

  // Fallback toast for the "View profile" action — Req 15.4 §35.
  // A dedicated profile card modal will land in a future iteration;
  // for now we surface a non-intrusive notification so users know the
  // click was registered.
  Effect::new(move |_| {
    if let Some(target) = app_state.pending_profile.get() {
      let name = app_state.room_members.with(|map| {
        map
          .get(&room_id.get())
          .and_then(|list| list.iter().find(|m| m.user_id == target).cloned())
          .map(|m| m.nickname)
          .unwrap_or_else(|| target.to_string())
      });
      let template = t_string!(i18n, room.view_profile_pending);
      toast.show_info_message_with_key(
        "ROM2200",
        "room.view_profile_pending",
        &interpolate_name(template, &name),
      );
      app_state.pending_profile.set(None);
    }
  });

  let actor_id = Signal::derive(move || {
    app_state
      .auth
      .with(|a| a.as_ref().map(|a| a.user_id.clone()))
  });

  let sorted_members = Memo::new(move |_| {
    let rid = room_id.get();
    app_state
      .room_members
      .with(|map| map.get(&rid).cloned().unwrap_or_default())
  });

  let actor_role = Memo::new(move |_| {
    let id = actor_id.get();
    let members = sorted_members.get();
    match id {
      Some(user_id) => current_role(&members, &user_id),
      None => message::types::RoomRole::Member,
    }
  });

  // Look up the current room's metadata so the Owner can open the
  // settings modal (Req 4.5). Returns None until the room list has
  // been pushed by the server.
  let current_room = Memo::new(move |_| {
    let rid = room_id.get();
    app_state
      .rooms
      .with(|list| list.iter().find(|r| r.room_id == rid).cloned())
  });
  let is_owner = Memo::new(move |_| actor_role.get() == message::types::RoomRole::Owner);

  let filtered_members = Memo::new(move |_| {
    let q = query.get();
    let members = sorted_members.get();
    filter_members(members, &q)
  });

  let match_count = Memo::new(move |_| filtered_members.get().len());

  let display_name_for = move |target: &UserId| -> String {
    sorted_members
      .get()
      .into_iter()
      .find(|m| &m.user_id == target)
      .map_or_else(
        || target.to_string(),
        |m| {
          if m.nickname.is_empty() {
            m.user_id.to_string()
          } else {
            m.nickname.clone()
          }
        },
      )
  };

  let dispatch_action = {
    let signaling = signaling.clone();
    move |target: UserId, action: MemberAction, duration_secs: Option<u64>| {
      let room = room_id.get();
      let result = match action {
        MemberAction::Kick => signaling.send_kick_member(room, target),
        MemberAction::Mute => signaling.send_mute_member(room, target, duration_secs),
        MemberAction::Unmute => signaling.send_unmute_member(room, target),
        MemberAction::Ban => signaling.send_ban_member(room, target),
        MemberAction::Unban => signaling.send_unban_member(room, target),
        MemberAction::Promote => signaling.send_promote_admin(room, target),
        MemberAction::Demote => signaling.send_demote_admin(room, target),
        MemberAction::TransferOwnership => signaling.send_transfer_ownership(room, target),
        // Non-moderation actions never reach `dispatch_action` because
        // `on_action` handles them locally; this branch keeps the
        // match exhaustive.
        MemberAction::ViewProfile
        | MemberAction::StartDirectMessage
        | MemberAction::Mention
        | MemberAction::Leave => {
          return;
        }
      };
      if let Err(e) = result {
        web_sys::console::warn_1(&format!("[room] Moderation action failed: {e}").into());
        toast.show_error_message_with_key("ROM110", "error.rom110", t_string!(i18n, error.rom110));
      }
    }
  };

  let on_action = Callback::new({
    let dispatch_action = dispatch_action.clone();
    let signaling_for_leave = signaling.clone();
    move |(target, action): (UserId, MemberAction)| {
      let target_name = display_name_for(&target);
      let meta = member_action_meta(action);
      // Confirm dialog wins over the mute picker so the developer
      // cannot accidentally mark an action as both `needs_confirm` and
      // `needs_duration_picker` and have one silently swallow the
      // other.
      if meta.needs_confirm {
        pending_confirm.set(Some(PendingConfirm {
          action,
          target,
          target_name,
        }));
        return;
      }
      if meta.needs_duration_picker {
        pending_mute.set(Some((target, target_name)));
        return;
      }
      // Immediate / non-moderation paths.
      match action {
        MemberAction::Unmute => dispatch_action(target, action, None),
        MemberAction::ViewProfile => {
          app_state.pending_profile.set(Some(target));
        }
        MemberAction::StartDirectMessage => {
          app_state
            .active_conversation
            .set(Some(crate::state::ConversationId::Direct(target)));
        }
        MemberAction::Mention => {
          app_state.pending_mention.set(Some(target_name));
        }
        MemberAction::Leave => {
          let _ = signaling_for_leave.send_leave_room(room_id.get());
          app_state.active_conversation.set(None);
        }
        // Any moderation action that reaches this point is a
        // configuration bug — the metadata says it should not appear
        // here. Fall back to the immediate dispatch path so behaviour
        // remains predictable.
        other => dispatch_action(target, other, None),
      }
    }
  });

  let action_title = move |action: MemberAction| -> String {
    match action {
      MemberAction::Kick => t_string!(i18n, room.kick).to_string(),
      MemberAction::Ban => t_string!(i18n, room.ban).to_string(),
      MemberAction::Unban => t_string!(i18n, room.unban).to_string(),
      MemberAction::Promote => t_string!(i18n, room.promote).to_string(),
      MemberAction::Demote => t_string!(i18n, room.demote).to_string(),
      MemberAction::TransferOwnership => t_string!(i18n, room.transfer_ownership).to_string(),
      MemberAction::Mute | MemberAction::Unmute => t_string!(i18n, room.mute).to_string(),
      MemberAction::Leave => t_string!(i18n, room.leave).to_string(),
      MemberAction::ViewProfile => t_string!(i18n, room.view_profile).to_string(),
      MemberAction::StartDirectMessage => t_string!(i18n, room.direct_message).to_string(),
      MemberAction::Mention => t_string!(i18n, room.mention_in_chat).to_string(),
    }
  };
  let action_description = move |action: MemberAction, target_name: &str| -> String {
    let template = match action {
      MemberAction::Kick => t_string!(i18n, room.confirm_kick),
      MemberAction::Ban => t_string!(i18n, room.confirm_ban),
      MemberAction::Unban => t_string!(i18n, room.confirm_unban),
      MemberAction::Promote => t_string!(i18n, room.confirm_promote),
      MemberAction::Demote => t_string!(i18n, room.confirm_demote),
      MemberAction::TransferOwnership => t_string!(i18n, room.confirm_transfer),
      MemberAction::Leave => t_string!(i18n, room.confirm_leave),
      MemberAction::Mute
      | MemberAction::Unmute
      | MemberAction::ViewProfile
      | MemberAction::StartDirectMessage
      | MemberAction::Mention => t_string!(i18n, room.confirm_generic),
    };
    interpolate_name(template, target_name)
  };

  let confirm_title = Memo::new(move |_| {
    pending_confirm
      .with(|p| p.as_ref().map(|p| action_title(p.action)))
      .unwrap_or_default()
  });
  let confirm_body = Memo::new(move |_| {
    pending_confirm
      .with(|p| {
        p.as_ref()
          .map(|p| action_description(p.action, &p.target_name))
      })
      .unwrap_or_default()
  });
  let confirm_label = Memo::new(move |_| {
    pending_confirm
      .with(|p| p.as_ref().map(|p| action_title(p.action)))
      .unwrap_or_default()
  });
  let confirm_tone = Memo::new(move |_| {
    pending_confirm
      .with(|p| p.as_ref().map(|p| confirm_tone_for(p.action)))
      .unwrap_or(ConfirmTone::Neutral)
  });

  let on_confirm = Callback::new({
    let dispatch_action = dispatch_action.clone();
    move |()| {
      if let Some(p) = pending_confirm.get() {
        dispatch_action(p.target, p.action, None);
      }
      pending_confirm.set(None);
    }
  });

  let on_confirm_cancel = Callback::new(move |()| pending_confirm.set(None));

  let on_mute_pick = Callback::new({
    let dispatch_action = dispatch_action.clone();
    move |secs: Option<u64>| {
      if let Some((target, _)) = pending_mute.get() {
        dispatch_action(target, MemberAction::Mute, secs);
      }
      pending_mute.set(None);
    }
  });
  let on_mute_cancel = Callback::new(move |()| pending_mute.set(None));

  view! {
    <aside class="room-member-list" data-testid="room-member-list">
      <header class="room-member-list__header">
        <h3 class="room-member-list__title">{t!(i18n, room.members)}</h3>
        <span class="room-member-list__count" data-testid="room-member-count">
          {move || format!("{}", match_count.get())}
        </span>
        <Show when=move || is_owner.get() && current_room.get().is_some()>
          <button
            type="button"
            class="btn btn--ghost room-member-list__invite"
            aria-label=move || t_string!(i18n, room.invite_send_label)
            on:click=move |_| invite_open.set(true)
            data-testid="room-member-list-invite"
          >
            "✉ "{t!(i18n, room.invite_to_room)}
          </button>
          <button
            type="button"
            class="btn btn--ghost room-member-list__settings"
            aria-label=move || t_string!(i18n, room.settings_open)
            on:click=move |_| settings_open.set(true)
            data-testid="room-member-list-settings"
          >
            "⚙ "{t!(i18n, room.settings)}
          </button>
        </Show>
      </header>
      <div class="room-member-list__search">
        <input
          type="search"
          class="input"
          placeholder=move || t_string!(i18n, common.search)
          prop:value=move || query.get()
          on:input=move |ev| query.set(event_target_value(&ev))
          aria-label=move || t_string!(i18n, room.member_search)
          data-testid="room-member-search"
        />
        <Show when=move || !query.get().is_empty()>
          <span class="room-member-list__result-count" aria-live="polite">
            {move || format!("{} {}", match_count.get(), t_string!(i18n, room.results_found))}
          </span>
        </Show>
      </div>

      <Show when=move || { actor_role.get() > message::types::RoomRole::Member }>
        <div class="room-member-list__batch-bar" role="toolbar"
          aria-label=move || t_string!(i18n, room.batch_toolbar_aria)>
          <button
            type="button"
            class="btn btn--ghost"
            on:click=move |_| {
              batch_mode.update(|v| *v = !*v);
              if !batch_mode.get_untracked() {
                batch_selection.set(Vec::new());
              }
            }
            data-testid="room-member-batch-toggle"
          >
            {move || if batch_mode.get() {
              t_string!(i18n, room.batch_mode_exit).to_string()
            } else {
              t_string!(i18n, room.batch_mode).to_string()
            }}
          </button>
          <Show when=move || batch_mode.get()>
            <span class="room-member-list__batch-count">
              {move || format!("{} / {BATCH_MAX}", batch_selection.with(Vec::len))}
            </span>
            <button
              type="button"
              class="btn btn--ghost"
              disabled=move || batch_selection.with(Vec::is_empty)
              on:click=move |_| batch_mute_open.set(true)
              data-testid="room-member-batch-mute"
            >
              {t!(i18n, room.batch_mute_selected)}
            </button>
          </Show>
          <button
            type="button"
            class="btn btn--ghost"
            on:click=move |_| history_open.set(true)
            data-testid="room-member-history-open"
          >
            {t!(i18n, room.history_open)}
          </button>
        </div>
      </Show>

      <Show
        when=move || !filtered_members.get().is_empty()
        fallback=move || view! {
          <p class="room-member-list__empty">
            {move || {
              let q = query.get();
              if q.is_empty() {
                t_string!(i18n, room.empty_members).to_string()
              } else {
                interpolate_query(t_string!(i18n, room.no_results_for), &q)
              }
            }}
          </p>
        }
      >
        <ul class="room-member-list__list" role="list">
          <For
            each=move || filtered_members.get()
            key=|m: &MemberInfo| m.user_id.clone()
            children=move |member: MemberInfo| {
              // Derive a reactive Signal<MemberInfo> from app state so
              // role / mute / nickname mutations reach this row even
              // after the initial render (Req 15.5.39, BUG-02 fix).
              let row_user_id = member.user_id.clone();
              let initial = member.clone();
              let row_user_id_for_member = row_user_id.clone();
              let member_sig: Signal<MemberInfo> = Signal::derive(move || {
                let rid = room_id.get();
                let target = row_user_id_for_member.clone();
                app_state.room_members.with(|map| {
                  map
                    .get(&rid)
                    .and_then(|list| list.iter().find(|m| m.user_id == target).cloned())
                    .unwrap_or_else(|| initial.clone())
                })
              });
              // Compute username disambiguation badge text whenever
              // the member set or this row's nickname changes
              // (Req 15.1.5 / 15.1.6). The badge falls back to a short
              // 8-char prefix of the user_id since `MemberInfo` does
              // not currently carry the canonical username.
              let row_user_id_for_badge = row_user_id.clone();
              let username_badge: Signal<Option<String>> = Signal::derive(move || {
                let rid = room_id.get();
                let target = row_user_id_for_badge.clone();
                app_state.room_members.with(|map| {
                  let list = map.get(&rid)?;
                  let me = list.iter().find(|m| m.user_id == target)?;
                  if !member_needs_username_badge(me, list) {
                    return None;
                  }
                  let id_str = me.user_id.to_string();
                  Some(id_str.chars().take(8).collect::<String>())
                })
              });
              let row_user_id_for_check = row_user_id.clone();
              let row_user_id_for_toggle = row_user_id.clone();
              let is_selected = Signal::derive(move || {
                let target = row_user_id_for_check.clone();
                batch_selection.with(|sel| sel.contains(&target))
              });
              let toggle_select = move |_| {
                let target = row_user_id_for_toggle.clone();
                batch_selection.update(|sel| {
                  if let Some(pos) = sel.iter().position(|u| u == &target) {
                    sel.remove(pos);
                  } else if sel.len() < BATCH_MAX {
                    sel.push(target);
                  }
                });
              };
              view! {
                <div class="room-member-list__row-wrapper">
                  <Show when=move || batch_mode.get()>
                    <input
                      type="checkbox"
                      class="room-member-list__batch-checkbox"
                      aria-label=move || t_string!(i18n, room.batch_select_label)
                      prop:checked=move || is_selected.get()
                      on:change=toggle_select.clone()
                      data-testid="room-member-batch-checkbox"
                    />
                  </Show>
                  <MemberRow
                    member=member_sig
                    actor_role=Signal::derive(move || actor_role.get())
                    actor_id=Signal::derive(move || actor_id.get())
                    query=Signal::derive(move || query.get())
                    username_badge=username_badge
                    on_action=on_action
                  />
                </div>
              }
            }
          />
        </ul>
      </Show>

      <Show when=move || pending_confirm.get().is_some()>
        <ConfirmDialog
          title=Signal::derive(move || confirm_title.get())
          description=Signal::derive(move || confirm_body.get())
          confirm_label=Signal::derive(move || confirm_label.get())
        tone=Signal::derive(move || confirm_tone.get())
          on_confirm=on_confirm
          on_cancel=on_confirm_cancel
        />
      </Show>

      <Show when=move || pending_mute.get().is_some()>
        <MuteDurationPicker
          target_name=Signal::derive(move || {
            pending_mute
              .with(|p| p.as_ref().map(|(_, name)| name.clone()))
              .unwrap_or_default()
          })
          on_pick=on_mute_pick
          on_cancel=on_mute_cancel
        />
      </Show>

      <Show when=move || batch_mute_open.get()>
        <MuteDurationPicker
          target_name=Signal::derive(move || {
            let template = t_string!(i18n, room.batch_members_label);
            interpolate_count(template, batch_selection.with(Vec::len))
          })
          on_pick=Callback::new({
            let signaling = signaling.clone();
            move |secs| {
              let room = room_id.get();
              let targets = batch_selection.get_untracked();
              if targets.len() > BATCH_MAX {
                toast.show_error_message_with_key(
                  "ROM2300",
                  "room.batch_max_reached",
                  t_string!(i18n, room.batch_max_reached),
                );
                batch_mute_open.set(false);
                return;
              }
              let mut errors = 0_usize;
              for target in &targets {
                if signaling
                  .send_mute_member(room.clone(), target.clone(), secs)
                  .is_err()
                {
                  errors += 1;
                }
              }
              if errors > 0 {
                web_sys::console::warn_1(
                  &format!("[room] {errors} batch operations failed").into(),
                );
                toast.show_error_message_with_key(
                  "ROM111",
                  "error.rom111",
                  t_string!(i18n, error.rom111),
                );
              }
              let template = t_string!(i18n, room.batch_action_succeeded);
              toast.show_info_message_with_key(
                "ROM2301",
                "room.batch_action_succeeded",
                &interpolate_count(template, targets.len()),
              );
              batch_mute_open.set(false);
              batch_selection.set(Vec::new());
              batch_mode.set(false);
            }
          })
          on_cancel=Callback::new(move |()| batch_mute_open.set(false))
        />
      </Show>

      <Show when=move || history_open.get()>
        <MemberHistoryPanel
          room_id=room_id
          on_close=Callback::new(move |()| history_open.set(false))
        />
      </Show>

      <Show when=move || invite_open.get() && current_room.get().is_some()>
        <InviteMemberModal
          open=invite_open
          room=Signal::derive(move || current_room.get().unwrap_or_else(|| {
            message::types::RoomInfo::new(
              room_id.get(),
              String::new(),
              message::types::RoomType::Chat,
              actor_id.get().unwrap_or_else(message::UserId::new),
            )
          }))
        />
      </Show>

      <Show when=move || settings_open.get() && current_room.get().is_some()>
        <RoomSettingsModal
          open=settings_open
          room=Signal::derive(move || current_room.get().unwrap_or_else(|| {
            // Should never run because <Show> guards is_some(); fall
            // back to a default to keep the type system happy.
            message::types::RoomInfo::new(
              room_id.get(),
              String::new(),
              message::types::RoomType::Chat,
              actor_id.get().unwrap_or_else(message::UserId::new),
            )
          }))
        />
      </Show>
    </aside>
  }
}

/// Map the action onto a [`ConfirmTone`] so destructive actions get a
/// red CTA button. Backed by [`member_action_meta`] so a single source
/// of truth drives both the menu rendering and the confirmation tone.
const fn confirm_tone_for(action: MemberAction) -> ConfirmTone {
  if member_action_meta(action).destructive {
    ConfirmTone::Destructive
  } else {
    ConfirmTone::Neutral
  }
}
