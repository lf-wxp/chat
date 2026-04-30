//! Sidebar panel listing all currently online users (Req 9.1).
//!
//! Provides:
//!
//! - A real-time list driven by `AppState::online_users` (which is
//!   refreshed by every `UserListUpdate` / `UserStatusChange`).
//! - A search input (Req 9.11) that fuzzy-filters the list by
//!   username or nickname using a case-insensitive substring match.
//! - A multi-select toggle (Req 9.12) that flips the row buttons
//!   into checkboxes and reveals the "Send invitations" footer button.
//! - Per-row badges describing the connection state (Connected /
//!   Inviting / Connecting / Blocked) so the caller can decide how
//!   to act.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::UserId;
use message::types::{UserInfo, UserStatus};

use crate::blacklist::use_blacklist_state;
use crate::components::discovery::MultiInvitePanel;
use crate::i18n;
use crate::identicon::generate_identicon_data_uri;
use crate::invite::{InviteStatus, use_invite_manager};
use crate::state::{ConversationId, use_app_state};
use crate::webrtc::try_use_webrtc_manager;
use icondata as i;
use leptos_icons::Icon;

/// Online users sidebar panel.
#[component]
pub fn OnlineUsersPanel(
  /// Reactive state containing the user id whose info card is open.
  /// Set by clicks on a row; cleared when the modal closes.
  selected: RwSignal<Option<UserId>>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();

  let query = RwSignal::new(String::new());
  let multi_select = RwSignal::new(false);
  let selected_targets = RwSignal::new(Vec::<UserId>::new());

  let online_users = app_state.online_users;
  let me = Memo::new(move |_| {
    app_state
      .auth
      .with(|a| a.as_ref().map(|a| a.user_id.clone()))
  });

  let visible_users = Memo::new(move |_| {
    let q = query.get().to_lowercase();
    let mine = me.get_untracked();
    online_users.with(|list| {
      list
        .iter()
        .filter(|u| Some(&u.user_id) != mine.as_ref())
        .filter(|u| {
          if q.is_empty() {
            return true;
          }
          u.username.to_lowercase().contains(&q) || u.nickname.to_lowercase().contains(&q)
        })
        .cloned()
        .collect::<Vec<_>>()
    })
  });

  // Toggling multi-select off should also clear any pre-existing
  // selections so the next "Multi" press starts fresh.
  Effect::new(move |_| {
    if !multi_select.get() {
      selected_targets.set(Vec::new());
    }
  });

  let header_label = move || t_string!(i18n, discovery.online_users);
  let toggle_multi_label = move || {
    if multi_select.get() {
      t_string!(i18n, common.cancel)
    } else {
      t_string!(i18n, discovery.multi_invite)
    }
  };

  view! {
    <section
      class="discovery-panel"
      aria-label=move || t_string!(i18n, discovery.online_users)
      data-testid="online-users-panel"
    >
      <header class="discovery-panel__header">
        <h2 class="discovery-panel__title">{header_label}</h2>
        <button
          type="button"
          class=move || {
            if multi_select.get() {
              "btn btn--ghost discovery-panel__multi is-active"
            } else {
              "btn btn--ghost discovery-panel__multi"
            }
          }
          aria-pressed=move || multi_select.get().to_string()
          on:click=move |_| multi_select.update(|v| *v = !*v)
          data-testid="online-users-multi-toggle"
        >
          {toggle_multi_label}
        </button>
      </header>

      <div class="discovery-panel__search">
        <input
          type="search"
          class="discovery-panel__search-input"
          placeholder=move || t_string!(i18n, common.search)
          aria-label=move || t_string!(i18n, common.search)
          prop:value=move || query.get()
          on:input=move |ev| query.set(event_target_value(&ev))
          data-testid="online-users-search"
        />
      </div>

      <ul class="discovery-panel__list" role="list">
        <Show
          when=move || !visible_users.get().is_empty()
          fallback=move || view! {
            <li class="discovery-panel__empty">{t!(i18n, discovery.empty_list)}</li>
          }
        >
          <For
            each=move || visible_users.get()
            key=|u: &UserInfo| u.user_id.clone()
            children=move |user: UserInfo| {
              view! {
                <UserRow
                  user=user
                  multi_select=multi_select
                  selected_targets=selected_targets
                  selected_card=selected
                />
              }
            }
          />
        </Show>
      </ul>

      <Show when=move || multi_select.get()>
        <MultiInvitePanel
          selected_targets=selected_targets
          on_done=Callback::new(move |()| {
            multi_select.set(false);
          })
        />
      </Show>
    </section>
  }
}

/// Single online-user row. Extracted as a component so each row owns a
/// minimal reactive scope (one closure per status memo) — the parent
/// `<For/>` keys ensure rows do not unnecessarily rebuild.
///
/// ## Opt-4.1 — `RowSnapshot`
///
/// All four reactive inputs (blacklist / outbound invite state /
/// connected peers / multi-select membership) are collapsed into one
/// [`RowSnapshot`] held by a single `Memo`. Previously each row spawned
/// four `Memo`s × N rows, which added up on large online-user lists.
/// The snapshot recomputes as a single unit and the view closures below
/// each take a single `snapshot.get()` read.
#[derive(Clone, PartialEq, Eq)]
struct RowSnapshot {
  blocked: bool,
  invite: Option<InviteStatus>,
  connected: bool,
  selected: bool,
}

#[component]
fn UserRow(
  user: UserInfo,
  multi_select: RwSignal<bool>,
  selected_targets: RwSignal<Vec<UserId>>,
  selected_card: RwSignal<Option<UserId>>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let blacklist = use_blacklist_state();
  let invite_mgr = use_invite_manager();

  let user_id = user.user_id.clone();
  let display = if user.nickname.is_empty() {
    user.username.clone()
  } else {
    user.nickname.clone()
  };
  let status = user.status;
  let avatar = user
    .avatar_url
    .clone()
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| generate_identicon_data_uri(&user.username));

  let snapshot = {
    let id = user_id.clone();
    let bl = blacklist.clone();
    let mgr = invite_mgr.clone();
    Memo::new(move |_| RowSnapshot {
      blocked: bl.is_blocked(&id),
      invite: mgr.outbound_status(&id),
      connected: try_use_webrtc_manager()
        .map(|m| m.connected_peers().contains(&id))
        .unwrap_or(false),
      selected: selected_targets.with(|s| s.contains(&id)),
    })
  };

  let display_for_card = display.clone();
  let user_id_for_card = user_id.clone();
  let user_id_for_chat = user_id.clone();
  let user_id_for_toggle = user_id.clone();

  let on_open_card = move |_| {
    if multi_select.get_untracked() {
      // In multi-select mode rows toggle selection instead of opening
      // the info card.
      let id = user_id_for_toggle.clone();
      selected_targets.update(|list| {
        if let Some(idx) = list.iter().position(|i| i == &id) {
          list.remove(idx);
        } else {
          list.push(id);
        }
      });
      return;
    }
    if snapshot.get_untracked().connected {
      // Already-connected users open the chat directly (Req 9.10).
      app_state
        .active_conversation
        .set(Some(ConversationId::Direct(user_id_for_chat.clone())));
      return;
    }
    selected_card.set(Some(user_id_for_card.clone()));
  };

  let row_class = move || {
    let snap = snapshot.get();
    let mut classes = String::from("discovery-row");
    if snap.connected {
      classes.push_str(" discovery-row--connected");
    }
    if snap.blocked {
      classes.push_str(" discovery-row--blocked");
    }
    match snap.invite {
      Some(InviteStatus::Pending) => classes.push_str(" discovery-row--pending"),
      Some(InviteStatus::Connecting) => classes.push_str(" discovery-row--connecting"),
      None => {}
    }
    if snap.selected {
      classes.push_str(" discovery-row--selected");
    }
    classes
  };

  let status_dot_class = match status {
    UserStatus::Online => "discovery-row__status discovery-row__status--online",
    UserStatus::Busy => "discovery-row__status discovery-row__status--busy",
    UserStatus::Away => "discovery-row__status discovery-row__status--away",
    UserStatus::Offline => "discovery-row__status discovery-row__status--offline",
  };

  let badge_label = move || {
    let snap = snapshot.get();
    if snap.blocked {
      Some(t_string!(i18n, discovery.blocked))
    } else if snap.connected {
      Some(t_string!(i18n, discovery.connected))
    } else {
      match snap.invite {
        Some(InviteStatus::Pending) => Some(t_string!(i18n, discovery.inviting)),
        Some(InviteStatus::Connecting) => Some(t_string!(i18n, discovery.connecting_please_wait)),
        None => None,
      }
    }
  };

  // Bug-5 fix: provide a tooltip on the row button so the reason a
  // user cannot be invited is discoverable without opening the card.
  let display_for_title = display_for_card.clone();
  let row_title = move || -> String {
    let snap = snapshot.get();
    if snap.blocked {
      t_string!(i18n, discovery.blocked_tooltip).to_string()
    } else {
      match snap.invite {
        Some(InviteStatus::Pending) => {
          t_string!(i18n, discovery.invite_pending_tooltip).to_string()
        }
        Some(InviteStatus::Connecting) => {
          t_string!(i18n, discovery.connecting_please_wait).to_string()
        }
        None => display_for_title.clone(),
      }
    }
  };

  view! {
    <li class=row_class data-testid="online-user-row">
      <button
        type="button"
        class="discovery-row__button"
        on:click=on_open_card
        aria-label=display_for_card.clone()
        title=row_title
      >
        <Show when=move || multi_select.get()>
          <span class="discovery-row__check" aria-hidden="true">
            {move || if snapshot.get().selected {
              view! { <Icon icon=i::LuSquareCheck /> }.into_any()
            } else {
              view! { <Icon icon=i::LuSquare /> }.into_any()
            }}
          </span>
        </Show>
        <img
          class="discovery-row__avatar"
          src=avatar.clone()
          alt=""
          width="36"
          height="36"
        />
        <span class="discovery-row__info">
          <span class="discovery-row__name">{display.clone()}</span>
          <span class="discovery-row__meta">
            <span class=status_dot_class aria-hidden="true"></span>
            <span class="discovery-row__status-label">{format!("{status:?}")}</span>
          </span>
        </span>
        <Show when=move || badge_label().is_some()>
          <span class="discovery-row__badge">{move || badge_label().unwrap_or_default()}</span>
        </Show>
      </button>
    </li>
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn user(seed: &str, status: UserStatus) -> UserInfo {
    UserInfo {
      user_id: message::UserId::from_uuid(uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_DNS,
        seed.as_bytes(),
      )),
      username: seed.to_string(),
      nickname: format!("{seed} display"),
      status,
      avatar_url: None,
      bio: String::new(),
      created_at_nanos: 0,
      last_seen_nanos: 0,
    }
  }

  #[test]
  fn case_insensitive_filter_matches_username_and_nickname() {
    let users = [
      user("Alice", UserStatus::Online),
      user("bob", UserStatus::Offline),
      user("Carol", UserStatus::Busy),
    ];
    let query = "alice";
    let filtered: Vec<&UserInfo> = users
      .iter()
      .filter(|u| {
        u.username.to_lowercase().contains(query) || u.nickname.to_lowercase().contains(query)
      })
      .collect();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].username, "Alice");
  }

  #[test]
  fn empty_query_returns_all_users() {
    let users = [
      user("alice", UserStatus::Online),
      user("bob", UserStatus::Offline),
    ];
    let query = "";
    let filtered: Vec<&UserInfo> = users
      .iter()
      .filter(|u| {
        if query.is_empty() {
          return true;
        }
        u.username.to_lowercase().contains(query)
      })
      .collect();
    assert_eq!(filtered.len(), 2);
  }
}
