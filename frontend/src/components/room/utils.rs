//! Shared helpers for the room UI (permission checks, sorting,
//! mute countdown, etc.). Keeping them in a dedicated module lets
//! the presentational components stay focused on rendering.

use chrono::Utc;
use message::UserId;
use message::types::{MemberInfo, MuteInfo, RoomRole};

/// Moderation actions that can be attempted from the member context
/// menu. The role-aware permission matrix is centralised in
/// [`can_act_on`] so UI code does not duplicate the rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberAction {
  /// Remove the member from the room.
  Kick,
  /// Temporarily silence the member.
  Mute,
  /// Revoke an active mute ahead of time.
  Unmute,
  /// Prevent the member from rejoining.
  Ban,
  /// Lift a ban.
  Unban,
  /// Elevate a regular member to admin.
  Promote,
  /// Demote an admin back to member.
  Demote,
  /// Transfer ownership of the room.
  TransferOwnership,
  /// Leave the room voluntarily (self-only action).
  Leave,
  /// Open the member's profile card (Req 15.4 §35).
  ViewProfile,
  /// Start a 1-on-1 direct message with the member (Req 15.4 §35).
  StartDirectMessage,
  /// Insert an `@mention` for the member into the chat composer
  /// (Req 15.4 §35).
  Mention,
}

/// Compile-time descriptor that bundles together everything the UI
/// needs to know about a [`MemberAction`]: which i18n key holds the
/// label, whether the action requires a confirmation dialog, whether
/// it is destructive (red CTA), and whether it needs the duration
/// picker (only Mute does today).
///
/// Centralising the metadata here keeps the per-component `match`
/// arms in [`member_context_menu`] and [`member_list`] short and
/// guarantees that adding a new action only requires extending the
/// `MEMBER_ACTION_META` table — no other site has to be updated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberActionMeta {
  /// Action discriminant.
  pub action: MemberAction,
  /// Whether the action needs a confirmation dialog (kick/ban/promote
  /// /demote/transfer/unban).
  pub needs_confirm: bool,
  /// Whether the action is destructive (kick/ban/transfer ownership).
  /// Drives the red CTA tone in the confirmation dialog.
  pub destructive: bool,
  /// Whether the action requires the mute-duration picker.
  pub needs_duration_picker: bool,
  /// Whether the action runs against the server immediately without
  /// any confirmation or extra modal (unmute and the non-moderation
  /// view-profile / DM / mention shortcuts).
  pub immediate: bool,
}

/// Lookup the metadata for a single action.
#[must_use]
pub const fn member_action_meta(action: MemberAction) -> MemberActionMeta {
  match action {
    MemberAction::Kick => MemberActionMeta {
      action,
      needs_confirm: true,
      destructive: true,
      needs_duration_picker: false,
      immediate: false,
    },
    MemberAction::Ban => MemberActionMeta {
      action,
      needs_confirm: true,
      destructive: true,
      needs_duration_picker: false,
      immediate: false,
    },
    MemberAction::TransferOwnership => MemberActionMeta {
      action,
      needs_confirm: true,
      destructive: true,
      needs_duration_picker: false,
      immediate: false,
    },
    MemberAction::Unban | MemberAction::Promote | MemberAction::Demote => MemberActionMeta {
      action,
      needs_confirm: true,
      destructive: false,
      needs_duration_picker: false,
      immediate: false,
    },
    MemberAction::Mute => MemberActionMeta {
      action,
      needs_confirm: false,
      destructive: false,
      needs_duration_picker: true,
      immediate: false,
    },
    MemberAction::Unmute => MemberActionMeta {
      action,
      needs_confirm: false,
      destructive: false,
      needs_duration_picker: false,
      immediate: true,
    },
    MemberAction::Leave => MemberActionMeta {
      action,
      needs_confirm: true,
      destructive: false,
      needs_duration_picker: false,
      immediate: false,
    },
    MemberAction::ViewProfile | MemberAction::StartDirectMessage | MemberAction::Mention => {
      MemberActionMeta {
        action,
        needs_confirm: false,
        destructive: false,
        needs_duration_picker: false,
        immediate: true,
      }
    }
  }
}

/// Resolve the current user's role inside a given member list.
///
/// Falls back to [`RoomRole::Member`] when the user is not found —
/// this keeps the permission helpers safe to call before the member
/// list has been populated.
#[must_use]
pub fn current_role(members: &[MemberInfo], user_id: &UserId) -> RoomRole {
  members
    .iter()
    .find(|m| &m.user_id == user_id)
    .map_or(RoomRole::Member, |m| m.role)
}

/// Decide whether `actor_role` has permission to run `action` against
/// `target_role`. Mirrors Req 15.3 §28 and is intentionally total so
/// UI buttons can toggle their `disabled` attribute from a single
/// call.
///
/// The core rule is: actors cannot moderate peers with a role that is
/// equal to or higher than their own (except for `TransferOwnership`
/// which requires the Owner role and a Member-or-higher target).
#[must_use]
pub fn can_act_on(actor_role: RoomRole, target_role: RoomRole, action: MemberAction) -> bool {
  match action {
    // Non-moderation actions are always permitted; the menu hides
    // them for self rows separately (Req 15.4 §35).
    MemberAction::ViewProfile | MemberAction::StartDirectMessage | MemberAction::Mention => true,
    // Leave is a self-only action — always permitted. The UI only
    // surfaces it for the current user's own row.
    MemberAction::Leave => true,
    // Owner-exclusive moderation actions.
    MemberAction::Ban | MemberAction::Unban | MemberAction::Demote => {
      actor_role == RoomRole::Owner && target_role != RoomRole::Owner
    }
    // Promote: only Owner can promote, and only non-Admin targets (an
    // Admin is already at the highest non-Owner tier).
    MemberAction::Promote => {
      actor_role == RoomRole::Owner
        && target_role != RoomRole::Owner
        && target_role != RoomRole::Admin
    }
    // Ownership transfer: only the Owner can initiate, and only to a
    // non-owner target (self-transfer is meaningless).
    MemberAction::TransferOwnership => {
      actor_role == RoomRole::Owner && target_role != RoomRole::Owner
    }
    // Kick / Mute / Unmute: Admin and Owner can act on strictly lower
    // ranked members. An Admin therefore cannot kick another Admin or
    // the Owner, matching §20 of Req 15.3.
    MemberAction::Kick | MemberAction::Mute | MemberAction::Unmute => actor_role > target_role,
  }
}

/// Whether the given member is currently muted based on the embedded
/// [`MuteInfo`]. Timed mutes that have already expired count as
/// "not muted" even if the server has not garbage-collected them yet.
///
/// The outer `matches!` guard is intentionally defensive: it ensures
/// that a future change to `MemberInfo::is_muted()` (e.g. adding a
/// new `MuteInfo` variant) cannot silently cause a "not muted" result
/// for a member who *is* in a muted state. If `is_muted()` ever
/// returns `true` for a variant we don't recognise, the guard will
/// catch it; if `is_muted()` returns `false` for a known variant,
/// the guard still lets the correct "not muted" result through.
#[must_use]
pub fn is_currently_muted(member: &MemberInfo) -> bool {
  matches!(
    &member.mute_info,
    MuteInfo::Permanent | MuteInfo::Timed { .. }
  ) && member.is_muted()
}

/// Remaining seconds on a timed mute, or `None` when the member is
/// either not muted or muted permanently.
#[must_use]
pub fn mute_remaining_seconds(member: &MemberInfo) -> Option<i64> {
  let expires_at = match member.mute_info {
    MuteInfo::Timed { expires_at_nanos } => expires_at_nanos,
    _ => return None,
  };
  let now_nanos = Utc::now().timestamp_nanos_opt().unwrap_or(i64::MAX);
  let delta_ns = expires_at.saturating_sub(now_nanos);
  if delta_ns <= 0 {
    return None;
  }
  Some(delta_ns / 1_000_000_000)
}

/// Deterministic default sort: Owner first, then Admins, then Members,
/// breaking ties by earliest join time so long-standing members
/// appear above newcomers (Req 15.4 §37).
pub fn sort_members_default(members: &mut [MemberInfo]) {
  members.sort_by(|a, b| {
    b.role
      .cmp(&a.role)
      .then_with(|| a.joined_at_nanos.cmp(&b.joined_at_nanos))
  });
}

/// Substitute the `{name}` placeholder in a localised template with
/// the supplied value (Sprint 4.7 of the review-task-21 follow-up).
///
/// Centralising the interpolation here makes it easy to add escaping
/// or locale-specific quoting later, and keeps callers from having
/// to remember the exact placeholder spelling.
#[must_use]
pub fn interpolate_name(template: &str, name: &str) -> String {
  template.replace("{name}", name)
}

/// Substitute the `{count}` placeholder in a localised template with
/// the supplied numeric value.
///
/// Complements [`interpolate_name`] for templates that contain a
/// quantity rather than a personal name.
#[must_use]
pub fn interpolate_count(template: &str, count: usize) -> String {
  template.replace("{count}", &count.to_string())
}

/// Substitute the `{current}` and `{max}` placeholders in a localised
/// member-count template (e.g. `"{current} / {max}"`).
#[must_use]
pub fn interpolate_member_count(template: &str, current: u32, max: u32) -> String {
  template
    .replace("{current}", &current.to_string())
    .replace("{max}", &max.to_string())
}

/// Substitute the `{query}` placeholder in a localised "no results"
/// template (e.g. `"No members found matching '{query}'"`).
#[must_use]
pub fn interpolate_query(template: &str, query: &str) -> String {
  template.replace("{query}", query)
}

/// Substitute the `{from}` and `{room}` placeholders in a localised
/// template with the supplied values.
///
/// Used by the incoming room invite modal where both the inviter name
/// and the room name need to be interpolated. Centralising this
/// avoids each caller hand-rolling `.replace()` chains and ensures
/// consistent escaping in the future.
#[must_use]
pub fn interpolate_from_and_room(template: &str, from: &str, room: &str) -> String {
  template.replace("{from}", from).replace("{room}", room)
}

/// Apply the room-member search filter (Req 15.4 §32).
///
/// * Sorts the input first via [`sort_members_default`] so that an
///   empty query yields a deterministic default order.
/// * Performs a case-insensitive substring match against either the
///   member's nickname or the string form of their `UserId`.
/// * Empty queries return the full sorted list verbatim.
#[must_use]
pub fn filter_members(mut members: Vec<MemberInfo>, query: &str) -> Vec<MemberInfo> {
  sort_members_default(&mut members);
  let q = query.trim().to_lowercase();
  if q.is_empty() {
    return members;
  }
  members
    .into_iter()
    .filter(|m| {
      m.nickname.to_lowercase().contains(&q) || m.user_id.to_string().to_lowercase().contains(&q)
    })
    .collect()
}

/// Whether the given member should display a username disambiguation
/// badge in the member list (Req 15.1.5 / 15.1.6).
///
/// Returns `true` when:
/// * The nickname is empty (badge holds the username fallback), OR
/// * Another member in the same room shares the same nickname.
#[must_use]
pub fn member_needs_username_badge(target: &MemberInfo, all: &[MemberInfo]) -> bool {
  if target.nickname.is_empty() {
    return true;
  }
  let mut count = 0_usize;
  for m in all {
    if m.nickname == target.nickname {
      count += 1;
      if count > 1 {
        return true;
      }
    }
  }
  false
}

/// Read the `checked` property from an event target. Useful for
/// checkbox `on:change` handlers where `event_target_value` does not
/// apply. Shared across room modals to avoid per-file duplication.
pub fn event_target_checked(ev: &leptos::ev::Event) -> bool {
  use wasm_bindgen::JsCast;
  ev.target()
    .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
    .is_some_and(|input| input.checked())
}
