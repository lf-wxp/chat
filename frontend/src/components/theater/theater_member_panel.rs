//! Theater member panel (Req 12.7).
//!
//! Thin wrapper around the generic [`crate::components::MemberListPanel`]
//! that adds the theater-specific moderation shortcuts on top:
//!
//! * **Mute all** / **Unmute all** — owner-only toggle that flips
//!   [`TheaterState::all_muted`] locally and broadcasts the unified
//!   `TheaterMuteAll` signaling message so every viewer updates.
//! * Header with viewer count / 8 capacity.
//!
//! Per-member actions (kick / mute / unmute / transfer ownership)
//! are delegated to the shared room member panel so we do not
//! duplicate permission-matrix logic between Chat rooms and Theater
//! rooms (see Req 15.3).

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::types::RoomId;

use crate::components::room::MemberListPanel;
use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::signaling::use_signaling_client;
use crate::state::use_app_state;
use crate::theater::{TheaterRole, use_theater_state};

/// Render the theater member panel.
#[component]
pub fn TheaterMemberPanel(
  /// Room id of the active theater.
  #[prop(into)]
  room_id: Signal<RoomId>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let state = use_theater_state();
  let app = use_app_state();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();

  let is_owner = move || state.my_role.get() == TheaterRole::Owner;

  let viewer_count = Signal::derive(move || {
    let rid = room_id.get();
    app
      .room_members
      .with(|map| map.get(&rid).map_or(0, Vec::len))
  });

  let viewer_count_label = move || {
    t_string!(i18n, theater.members_count)
      .to_string()
      .replace("{count}", &viewer_count.get().to_string())
  };

  // `on:click` handlers must be `Fn` (Leptos may invoke them multiple
  // times). `SignalingClient` is `Clone` but not `Copy`, so we wrap
  // the mute-all handler in `Callback` which provides the necessary
  // `Fn` interior. `ErrorToastManager` is `Copy`, no extra clone
  // needed.
  let handle_mute_all = Callback::new({
    let signaling = signaling.clone();
    move |_: ()| {
      if !is_owner() {
        return;
      }
      let rid = room_id.get_untracked();
      // Flip the local signal immediately for responsive UX; on
      // signaling failure we roll back so the owner UI no longer
      // disagrees with the real server/viewer state.
      let next = !state.all_muted.get_untracked();
      state.all_muted.set(next);
      if let Err(err) = signaling.clone().send_theater_mute_all(rid) {
        state.all_muted.set(!next);
        toast.show_error_message("THR002", &err);
      }
    }
  });

  view! {
    <section
      class="theater-member-panel"
      aria-label=move || t_string!(i18n, theater.members_title)
      data-testid="theater-member-panel"
    >
      <header class="theater-member-panel__header">
        <h3 class="theater-member-panel__title">{t!(i18n, theater.members_title)}</h3>
        <span class="theater-member-panel__count" aria-live="polite">
          {viewer_count_label}
        </span>
      </header>

      <Show when=is_owner>
        <button
          type="button"
          class="btn btn--ghost theater-member-panel__mute-all"
          class:is-active=move || state.all_muted.get()
          on:click=move |_| handle_mute_all.run(())
          aria-pressed=move || state.all_muted.get().to_string()
          data-testid="theater-mute-all"
        >
          <Show
            when=move || state.all_muted.get()
            fallback=move || view! { <span>{t!(i18n, theater.mute_all)}</span> }
          >
            <span>{t!(i18n, theater.unmute_all)}</span>
          </Show>
        </button>
      </Show>

      <Show when=move || state.all_muted.get() && !is_owner()>
        <p class="theater-member-panel__mute-banner" role="note">
          {t!(i18n, theater.mute_all_active)}
        </p>
      </Show>

      <div class="theater-member-panel__list">
        <MemberListPanel room_id=room_id />
      </div>
    </section>
  }
}
