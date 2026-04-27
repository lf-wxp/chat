//! Auto-adapting grid of [`VideoTile`]s for an active call.
//!
//! The layout adapts to the number of participants via a data attribute
//! (`data-count`) that CSS grid rules key off. When a participant is
//! screen-sharing, that tile is rendered at "hero" size while the
//! others are displayed as picture-in-picture-style thumbnails.

use leptos::prelude::*;

use crate::call::use_call_signals;
use crate::components::call::VideoTile;
use crate::state::use_app_state;

/// Render the video grid for the active call.
#[component]
pub fn VideoGrid() -> impl IntoView {
  let signals = use_call_signals();
  let app_state = use_app_state();

  // Local participant view model (derived every render so the preview
  // picks up live track toggles and the screen-share swap).
  let local_tile = Memo::new(move |_| {
    let stream = signals.local_stream.get();
    let media = signals.local_media.get();
    let hero = media.screen_sharing;
    let name = app_state
      .auth
      .with(|auth| auth.as_ref().map(|a| a.nickname.clone()))
      .unwrap_or_else(|| "You".to_string());
    let user_id = app_state
      .current_user_id()
      .unwrap_or_else(|| message::UserId::from_uuid(uuid::Uuid::nil()));
    (user_id, name, stream, hero)
  });

  // Total participant count (including self) — used by the CSS grid
  // rules to pick the right column layout.
  let count_attr = Memo::new(move |_| signals.participants.with(|map| map.len() + 1));

  // P2-9 fix: removed the speaking-clear Effect that previously fought
  // with the VAD sweep at 100 Hz. The `speaking` flag is now driven
  // exclusively by `CallManager::sweep_vad`, and tiles render whatever
  // the participants signal currently reports — no per-render reset.

  view! {
    <div class="video-grid" data-count=move || count_attr.get()>
      {move || {
        let (uid, name, stream, hero) = local_tile.get();
        view! {
          <VideoTile
            user_id=uid
            display_name=name
            stream=stream
            speaking=false
            is_local=true
            hero=hero
          />
        }
      }}
      <For
        each=move || {
          signals
            .participants
            .with(|map| {
              let mut v: Vec<_> = map.values().cloned().collect();
              v.sort_by_key(|p| p.user_id.to_string());
              v
            })
        }
        key=|p| p.user_id.clone()
        children=move |p| {
          let name = app_state
            .online_users
            .with(|users| {
              users.iter().find(|u| u.user_id == p.user_id).map(|u| u.nickname.clone())
            })
            .unwrap_or_else(|| p.user_id.to_string());
          view! {
            <VideoTile
              user_id=p.user_id.clone()
              display_name=name
              stream=p.stream.clone()
              speaking=p.speaking
              is_local=false
              hero=p.screen_sharing
              mic_enabled=p.mic_enabled
              camera_enabled=p.camera_enabled
              reconnecting=p.reconnecting
            />
          }
        }
      />
    </div>
  }
}
