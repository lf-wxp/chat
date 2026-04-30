//! Contextual action menu rendered next to a room member row.
//!
//! The list of actions surfaced is computed by
//! [`super::utils::can_act_on`] so the UI never shows a button the
//! current user cannot actually perform.

use leptos::prelude::*;
use leptos_i18n::t_string;
use message::types::{MemberInfo, RoomRole};

use crate::components::room::popover_wrapper::PopoverWrapper;
use crate::components::room::utils::{MemberAction, can_act_on, is_currently_muted};
use crate::i18n;

/// Context menu rendered relative to the member row that owns it.
#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn MemberContextMenu(
  /// The member the actions will apply to.
  #[prop(into)]
  target: Signal<MemberInfo>,
  /// Current user's role inside the room.
  #[prop(into)]
  actor_role: Signal<RoomRole>,
  /// Whether this menu row belongs to the current user (self). Self
  /// rows only show the "Leave" action; all moderation options are
  /// hidden.
  #[prop(into)]
  is_self: Signal<bool>,
  /// Called when the user picks one of the moderation actions.
  on_pick: Callback<MemberAction>,
  /// Called when the menu should close without making a choice.
  on_close: Callback<()>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();

  let actions = Memo::new(move |_| {
    if is_self.get() {
      return vec![MemberAction::Leave];
    }
    let member = target.get();
    let role = actor_role.get();
    let mut items: Vec<MemberAction> = Vec::new();
    // Non-moderation actions appear first so the menu still has
    // useful entries even for regular members (Req 15.4 §35).
    items.push(MemberAction::ViewProfile);
    items.push(MemberAction::StartDirectMessage);
    items.push(MemberAction::Mention);
    for action in [
      MemberAction::Kick,
      MemberAction::Mute,
      MemberAction::Unmute,
      MemberAction::Ban,
      MemberAction::Unban,
      MemberAction::Promote,
      MemberAction::Demote,
      MemberAction::TransferOwnership,
    ] {
      if !can_act_on(role, member.role, action) {
        continue;
      }
      // Hide Mute/Unmute based on current mute state.
      if action == MemberAction::Mute && is_currently_muted(&member) {
        continue;
      }
      if action == MemberAction::Unmute && !is_currently_muted(&member) {
        continue;
      }
      // Hide Promote if the target is already Admin, Demote if they
      // are not.
      if action == MemberAction::Promote && member.role == RoomRole::Admin {
        continue;
      }
      if action == MemberAction::Demote && member.role != RoomRole::Admin {
        continue;
      }
      items.push(action);
    }
    items
  });

  let has_any = Memo::new(move |_| !actions.get().is_empty());

  view! {
    <Show when=move || has_any.get()>
      <PopoverWrapper on_close=on_close>
        <div
          class="room-member-menu"
          role="menu"
          aria-label=move || t_string!(i18n, room.member_actions)
          data-testid="room-member-menu"
        >
          <For
            each=move || actions.get()
            key=|action: &MemberAction| format!("{action:?}")
            children=move |action: MemberAction| {
              let label = match action {
                MemberAction::Kick => t_string!(i18n, room.kick),
                MemberAction::Mute => t_string!(i18n, room.mute),
                MemberAction::Unmute => t_string!(i18n, room.unmute),
                MemberAction::Ban => t_string!(i18n, room.ban),
                MemberAction::Unban => t_string!(i18n, room.unban),
                MemberAction::Promote => t_string!(i18n, room.promote),
                MemberAction::Demote => t_string!(i18n, room.demote),
                MemberAction::TransferOwnership => t_string!(i18n, room.transfer_ownership),
                MemberAction::Leave => t_string!(i18n, room.leave),
                MemberAction::ViewProfile => t_string!(i18n, room.view_profile),
                MemberAction::StartDirectMessage => t_string!(i18n, room.direct_message),
                MemberAction::Mention => t_string!(i18n, room.mention_in_chat),
              };
              let label_str = label.to_string();
              let aria = move || {
                let target_name = target.with(|m| {
                  if m.nickname.is_empty() {
                    m.user_id.to_string()
                  } else {
                    m.nickname.clone()
                  }
                });
                format!("{label_str} — {target_name}")
              };
              view! {
                <button
                  type="button"
                  class="room-member-menu__item"
                  class:room-member-menu__item--danger=matches!(
                    action,
                    MemberAction::Kick | MemberAction::Ban
                  )
                  role="menuitem"
                  aria-label=aria
                  on:click=move |_| {
                    on_pick.run(action);
                    on_close.run(());
                  }
                  data-testid="room-member-menu-item"
                >
                  {label}
                </button>
              }
            }
          />
        </div>
      </PopoverWrapper>
    </Show>
  }
}
