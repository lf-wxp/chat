//! Multi-invite footer panel rendered while the OnlineUsersPanel is in
//! multi-select mode (Req 9.12). Shows the count of selected targets
//! and a "Send invitations" button that emits a `MultiInvite` and
//! marks every target as pending in the invite manager.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::UserId;

use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::invite::use_invite_manager;
use crate::signaling::use_signaling_client;
use crate::state::use_app_state;

#[component]
pub fn MultiInvitePanel(
  selected_targets: RwSignal<Vec<UserId>>,
  /// Invoked after the multi-invite has been dispatched (or cancelled)
  /// so the parent can leave multi-select mode.
  on_done: Callback<()>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let invite_mgr = use_invite_manager();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();

  let count = Memo::new(move |_| selected_targets.with(Vec::len));
  let count_label = move || count.get().to_string();

  let resolve_display = {
    let online = app_state.online_users;
    move |id: &UserId| -> String {
      online.with_untracked(|list| {
        list
          .iter()
          .find(|u| u.user_id == *id)
          .map(|u| {
            if u.nickname.is_empty() {
              u.username.clone()
            } else {
              u.nickname.clone()
            }
          })
          .unwrap_or_else(|| id.to_string())
      })
    }
  };

  let on_send = {
    let invite_mgr = invite_mgr.clone();
    let signaling = signaling.clone();
    move |_| {
      let targets = selected_targets.get();
      if targets.is_empty() {
        return;
      }
      let batch_id = uuid::Uuid::new_v4();
      let payload: Vec<(UserId, String)> = targets
        .iter()
        .map(|id| (id.clone(), resolve_display(id)))
        .collect();
      let added = invite_mgr.track_multi_outbound(payload, batch_id);
      if added.is_empty() {
        // All targets already had pending invites — nothing to send.
        // Surface this to the user rather than silently no-op'ing
        // (Opt-6 fix).
        toast.show_info_message_with_key(
          "DSC904",
          "discovery.multi_invite_all_pending",
          "All selected users already have a pending invitation.",
        );
        on_done.run(());
        return;
      }
      if let Err(e) = signaling.send_multi_invite(added.clone()) {
        for id in &added {
          invite_mgr.cancel_outbound(id);
        }
        toast.show_error_message_with_key(
          "SIG001",
          "discovery.invite_failed",
          &format!("Failed to send invitations: {e}"),
        );
      }
      selected_targets.set(Vec::new());
      on_done.run(());
    }
  };

  view! {
    <footer class="discovery-panel__multi-footer" data-testid="multi-invite-footer">
      <span class="discovery-panel__multi-count">
        {move || t_string!(i18n, discovery.selected_label)}
        " "
        <strong>{count_label}</strong>
      </span>
      <button
        type="button"
        class="btn btn--primary discovery-panel__multi-send"
        prop:disabled=move || count.get() == 0
        on:click=on_send
        data-testid="multi-invite-send"
      >
        {t!(i18n, discovery.send_invitations)}
      </button>
    </footer>
  }
}
