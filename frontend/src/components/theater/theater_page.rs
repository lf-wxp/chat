//! Theater page — desktop + mobile layout (Req 12.8).
//!
//! Composes every theater sub-component into the final page surface:
//!
//! * Top bar: room title.
//! * Left: copyright notice + video player (with the danmaku canvas
//!   and subtitle overlay stacked on top) + playback controls
//!   (owner-only; also owns the fullscreen toggle) + danmaku
//!   composer.
//! * Right: tabbed panel — chat feed or viewer list.
//!
//! Side responsibilities:
//!
//! 1. Keep `TheaterState` in sync with the currently-active room
//!    (room id, display name, owner id, my role).
//! 2. Install / tear down the theater DataChannel handler on the
//!    WebRTC manager so incoming danmaku / subtitles / playback
//!    broadcasts flow into the state.
//!
//! Each responsibility is isolated inside its own `Effect` so
//! individual re-runs stay narrow and testable.

use std::cell::RefCell;
use std::rc::Rc;

use icondata as i;
use js_sys;
use leptos::html;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use message::datachannel::{Danmaku, DanmakuBatch, DataChannelMessage};
use message::types::{RoomInfo, RoomRole, RoomType};
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

use crate::components::theater::{
  CopyrightNotice, DanmakuCanvas, DanmakuInput, DanmakuSettingsPanel, SubtitleOverlay,
  SubtitleSettingsPanel, TheaterChatPanel, TheaterGraceBanner, TheaterMemberPanel,
  TheaterPlaybackControls, TheaterVideoPlayer,
};
use crate::i18n;
use crate::state::use_app_state;
use crate::theater::{
  TheaterRole, apply_theater_inbound, classify_theater_inbound, use_theater_state,
};
use crate::webrtc::{TheaterPeerEvent, try_use_webrtc_manager};

/// Right-side panel tab selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidePanelTab {
  Chat,
  Members,
}

/// Theater page root.
#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn TheaterPage(
  /// The theater room metadata captured from the global room list.
  /// The page derives its own reactive state from this snapshot and
  /// keeps `TheaterState` up to date via effects.
  #[prop(into)]
  room: Signal<RoomInfo>,
) -> impl IntoView {
  debug_assert_eq!(
    room.with_untracked(|r| r.room_type),
    RoomType::Theater,
    "TheaterPage must only be mounted for Theater rooms"
  );

  let state = use_theater_state();
  let app = use_app_state();
  let i18n = i18n::use_i18n();

  let active_tab = RwSignal::<SidePanelTab>::new(SidePanelTab::Chat);
  let section_ref: NodeRef<html::Section> = NodeRef::new();
  let video_ref: NodeRef<html::Video> = NodeRef::new();

  // ── Effect 1: mirror the active room into TheaterState ─────────────
  Effect::new(move |_| {
    let info = room.get();
    state.room_id.set(Some(info.room_id.clone()));
    state.room_name.set(info.name.clone());
    state.owner_id.set(Some(info.owner_id.clone()));
    let me = app.auth.with(|a| a.as_ref().map(|a| a.user_id.clone()));
    let role = match me {
      Some(ref my_id) if my_id == &info.owner_id => TheaterRole::Owner,
      Some(ref my_id) => {
        // Check the room member list for an Admin role assignment
        // so promoted admins retain moderation capabilities
        // (Req 12.7 §33-35 / Req 15.3).
        let is_admin = app.room_members.with(|map| {
          map
            .get(&info.room_id)
            .and_then(|members| members.iter().find(|m| &m.user_id == my_id))
            .is_some_and(|m| m.role == RoomRole::Admin)
        });
        if is_admin {
          TheaterRole::Admin
        } else {
          TheaterRole::Viewer
        }
      }
      None => TheaterRole::Viewer,
    };
    state.my_role.set(role);
  });

  // ── Effect 2: wire the WebRTC theater-message handler ──────────────
  // The handler is reinstalled on every mount so stale closures from a
  // previous session (capturing an old `state` snapshot) cannot fire.
  if let Some(manager) = try_use_webrtc_manager() {
    let manager_for_cleanup = manager.clone();
    manager.set_on_theater_message(move |_peer_id, msg| {
      // Classify + apply. Non-theater variants are impossible here
      // because the WebRTC layer only forwards the recognised set,
      // but we still unwrap defensively.
      if let Ok(inbound) = classify_theater_inbound(msg) {
        let _ = apply_theater_inbound(&state, inbound, |sender_id| {
          app.resolve_user_display_name(sender_id)
        });
      }
    });

    // Viewer-side remote track reception — Req 12.3. When the owner
    // pushes a MediaStream via `publish_local_stream` /
    // `publish_local_stream_to`, the browser fires `ontrack` on the
    // viewer's PeerConnection. This handler stores the stream in
    // `TheaterState::remote_stream` and flips `has_video_source` so
    // the `<video>` element mounts and the player can bind it.
    manager.set_on_theater_remote_track(move |_peer_id, stream| {
      let role = state.my_role.get_untracked();
      if role != TheaterRole::Owner {
        state.remote_stream.set(Some(stream));
        state.has_video_source.set(true);
      }
    });

    // Owner / viewer peer lifecycle — Req 12.2 §6a + 12.3 §12. The
    // callback runs alongside the call subsystem's handlers so the
    // theater page can flip `owner_reconnecting` (for viewers) and
    // publish the current MediaStream to late-joining viewers (for
    // owners) without displacing the call-side wiring.
    manager.set_on_theater_peer_event(move |peer_id, event| {
      let owner = state.owner_id.get_untracked();
      let role = state.my_role.get_untracked();
      match (role, event) {
        (TheaterRole::Owner, TheaterPeerEvent::Connected) => {
          // The auto-connect Effect establishes a DataChannel-only
          // connection first. Once ICE is fully connected, push the
          // local media stream (if available) via renegotiation.
          // A 500ms delay lets the ECDH handshake complete so the
          // DataChannel is ready for the renegotiation offer.
          if let Some(stream) = state.local_stream.get_untracked()
            && let Some(mgr) = try_use_webrtc_manager()
          {
            let pid = peer_id.clone();
            let s = stream.clone();
            let _ = crate::utils::set_timeout_once(500, move || {
              mgr.publish_local_stream_to(&pid, &s);
            });
          }
        }
        (_, TheaterPeerEvent::Connected) if owner.as_ref() == Some(&peer_id) => {
          // Viewer: owner peer just came back — clear the banner.
          state.owner_reconnecting.set(false);
          state
            .owner_grace_seconds
            .set(u8::try_from(crate::theater::GRACE_WINDOW_SECONDS).unwrap_or(u8::MAX));
        }
        (_, TheaterPeerEvent::Disconnected) if owner.as_ref() == Some(&peer_id) => {
          // Viewer: owner peer hit a transient ICE flap — surface
          // the 30s grace banner so the user can either wait or
          // leave (Req 12.2 §6a).
          // Guard: if the manager still has an active connection to
          // the owner, this event came from a stale/replaced PC and
          // should be ignored (SDP glare recovery path).
          if let Some(mgr) = try_use_webrtc_manager()
            && mgr.is_connected(&peer_id)
          {
            return;
          }
          state.owner_reconnecting.set(true);
        }
        (_, TheaterPeerEvent::Closed) if owner.as_ref() == Some(&peer_id) => {
          // Viewer: owner peer permanently gone. Leave the banner
          // up so the grace ticker flips to the "offline" CTA once
          // the window elapses; the grace helper will switch
          // messaging based on the remaining-seconds signal.
          // Guard: same as Disconnected — ignore if a replacement
          // connection is already active.
          if let Some(mgr) = try_use_webrtc_manager()
            && mgr.is_connected(&peer_id)
          {
            return;
          }
          state.owner_reconnecting.set(true);
          state.owner_grace_seconds.set(0);
        }
        _ => {}
      }
    });

    on_cleanup(move || {
      manager_for_cleanup.clear_on_theater_message();
      manager_for_cleanup.clear_on_theater_peer_event();
      manager_for_cleanup.clear_on_theater_remote_track();
    });
  }

  // ── Effect: owner auto-connect to late-joining viewers (Req 12.3 §12) ──
  // When a new viewer joins the theater room, the owner automatically
  // initiates a WebRTC connection (DataChannel only). Media tracks are
  // pushed separately via `publish_local_stream_to` in the
  // `TheaterPeerEvent::Connected` handler after the ICE connection is
  // fully established. This two-phase approach avoids ICE failures that
  // occur when media tracks are pre-attached to the initial SDP offer
  // in certain browser configurations.
  {
    let prev_members: Rc<RefCell<std::collections::HashSet<message::UserId>>> =
      Rc::new(RefCell::new(std::collections::HashSet::new()));
    Effect::new(move |_| {
      let role = state.my_role.get_untracked();
      if role != TheaterRole::Owner {
        return;
      }
      let Some(room_id) = state.room_id.get_untracked() else {
        return;
      };
      let current_members: std::collections::HashSet<message::UserId> =
        app.room_members.with(|map| {
          map
            .get(&room_id)
            .map(|list| list.iter().map(|m| m.user_id.clone()).collect())
            .unwrap_or_default()
        });
      let prev = prev_members.borrow().clone();
      let added: Vec<message::UserId> = current_members.difference(&prev).cloned().collect();
      *prev_members.borrow_mut() = current_members;

      if added.is_empty() {
        return;
      }
      let my_id = app
        .auth
        .with_untracked(|a| a.as_ref().map(|a| a.user_id.clone()));
      let Some(manager) = try_use_webrtc_manager() else {
        return;
      };
      for peer in added {
        if my_id.as_ref() == Some(&peer) {
          continue;
        }
        if manager.is_connected(&peer) {
          continue;
        }
        let mgr = manager.clone();
        leptos::task::spawn_local(async move {
          if let Err(e) = mgr.connect_to_peer(peer.clone()).await {
            web_sys::console::warn_1(
              &format!("[theater] auto-connect to viewer {} failed: {e}", peer).into(),
            );
          }
        });
      }
    });
  }

  // ── Effect: owner publishes stream to already-connected peers (late stream) ──
  // When the owner selects a large video file (e.g. MP4), `captureStream()`
  // is deferred until the `canplay` event fires. If viewers joined before
  // that point, their PeerConnections were established without media tracks.
  // This Effect watches `local_stream` and, when it transitions from None
  // to Some, publishes the stream to every already-connected peer via
  // `publish_local_stream`. The `onnegotiationneeded` handler on each PC
  // will then drive a renegotiation so the viewer receives the tracks.
  {
    let prev_had_stream: Rc<std::cell::Cell<bool>> = Rc::new(std::cell::Cell::new(false));
    Effect::new(move |_| {
      let role = state.my_role.get_untracked();
      if role != TheaterRole::Owner {
        return;
      }
      let stream = state.local_stream.get();
      let has_stream = stream.is_some();
      let had_before = prev_had_stream.get();
      prev_had_stream.set(has_stream);

      // Only act on the None → Some transition (first stream arrival).
      // The `apply_captured_stream` function already calls
      // `publish_local_stream` for the first stream, but that only
      // reaches peers whose connections are fully established at that
      // instant. This Effect fires reactively and catches any peers
      // that finished their SDP exchange in the meantime.
      if !had_before
        && has_stream
        && let Some(ref s) = stream
        && let Some(manager) = try_use_webrtc_manager()
      {
        // Small delay to let any in-flight SDP exchanges settle.
        let stream_clone = s.clone();
        let mgr = manager.clone();
        let _ = crate::utils::set_timeout_once(500, move || {
          mgr.publish_local_stream(&stream_clone);
        });
      }
    });
  }

  // ── Effect 3: owner-side danmaku relay (50 ms batch merge, Req 12.5 §28) ──
  // The owner collects inbound viewer danmaku in the shared batcher and
  // flushes them every 50 ms as **one merged `DanmakuBatch` per viewer**
  // so the per-peer send count is `M` rather than `N * M` for `N`
  // danmaku across `M` viewers.
  let relay_tick = set_interval_with_handle(
    move || {
      if state.my_role.get_untracked() != TheaterRole::Owner {
        return;
      }
      let Some(room_id) = state.room_id.get_untracked() else {
        return;
      };

      // 1. Relay danmaku as a single merged batch.
      let batch = state.with_danmaku_batcher::<Vec<Danmaku>>(|b| b.drain_batch());
      if !batch.is_empty()
        && let Some(manager) = try_use_webrtc_manager()
      {
        let frame = DanmakuBatch {
          room_id: room_id.clone(),
          entries: batch,
        };
        manager.broadcast_data_channel_message(&DataChannelMessage::DanmakuBatch(frame));
      }

      // 2. Relay theater chat bubbles that arrived from viewers.
      let pending = state.drain_chat_relay();
      if !pending.is_empty()
        && let Some(manager) = try_use_webrtc_manager()
      {
        for payload in pending {
          manager.broadcast_data_channel_message(&DataChannelMessage::TheaterChatText(payload));
        }
      }
    },
    std::time::Duration::from_millis(50),
  );
  if let Ok(handle) = relay_tick {
    on_cleanup(move || handle.clear());
  }

  // ── Effect 3b: owner-side resource monitor (1 s poll, Req 12.2 §4a) ──
  // Aggregates bufferedAmount across all viewer DataChannels and
  // triggers quality tier degradation / restoration based on the
  // thresholds defined in `resource_monitor`. Additionally invokes
  // `getStats()` to derive outbound bandwidth utilization.
  let monitor_snapshot = RwSignal::new(crate::theater::MonitorSnapshot::default());
  let bandwidth_snapshot = RwSignal::new(crate::theater::BandwidthSnapshot::default());
  let monitor_tick = set_interval_with_handle(
    move || {
      if state.my_role.get_untracked() != TheaterRole::Owner {
        return;
      }
      let Some(manager) = try_use_webrtc_manager() else {
        return;
      };
      // Aggregate bufferedAmount across all connected peers.
      let peers = manager.connected_peers();
      let aggregate: u32 = peers
        .iter()
        .filter_map(|peer_id| manager.buffered_amount(peer_id))
        .fold(0_u32, |acc, b| acc.saturating_add(b));

      // Update high-load warning signal (bufferedAmount heuristic).
      let buffer_high = crate::theater::is_high_load(aggregate);

      // Evaluate degradation / restoration.
      let current_tier = state.quality_tier.get_untracked();
      let mut action = crate::theater::MonitorAction::Hold;
      monitor_snapshot.update(|snap| {
        action = crate::theater::evaluate_tick(snap, aggregate, current_tier);
      });
      match action {
        crate::theater::MonitorAction::Degrade => {
          let next = crate::theater::degrade_tier(current_tier);
          state.quality_tier.set(next);
        }
        crate::theater::MonitorAction::Restore => {
          let next = crate::theater::restore_tier(current_tier);
          state.quality_tier.set(next);
        }
        crate::theater::MonitorAction::Hold => {}
      }

      // Async: collect outbound-rtp stats for bandwidth estimation.
      let mgr = manager.clone();
      leptos::task::spawn_local(async move {
        let stats = mgr.collect_stats().await;
        // Sum `bytesSent` across all outbound-rtp reports.
        let total_bytes_sent: u64 = stats
          .iter()
          .filter_map(|(_peer_id, report)| crate::webrtc::extract_outbound_bytes_sent(report))
          .sum();
        let now_ms = js_sys::Date::now() as u64;
        let mut estimate = crate::theater::BandwidthEstimate {
          throughput_bps: 0,
          utilization_percent: 0,
          is_saturated: false,
        };
        bandwidth_snapshot.update(|snap| {
          estimate = crate::theater::evaluate_bandwidth(
            snap,
            total_bytes_sent,
            now_ms,
            crate::theater::DEFAULT_CAPACITY_BPS,
          );
        });
        // Combine both signals: high-load if either buffer OR
        // bandwidth is saturated.
        state
          .owner_high_load
          .set(buffer_high || estimate.is_saturated);
      });
    },
    std::time::Duration::from_millis(1_000),
  );
  if let Ok(handle) = monitor_tick {
    on_cleanup(move || handle.clear());
  }

  // \u2500\u2500 Effect 3c: owner-side RAF frame-drop monitor (Req 12.2 §7a) \u2500\u2500
  // A cheap recursive `requestAnimationFrame` closure increments a
  // counter on every browser paint tick; the 1 Hz tick below reads
  // the counter, hands it to the pure `frame_drop_monitor::
  // evaluate_second` helper, and — when the drop rate has exceeded
  // 30 % for 10 seconds — steps the quality tier down one notch.
  //
  // The RAF closure owns non-`Send` `wasm_bindgen` bindings, so we
  // park it inside a `StoredValue::new_local` (Leptos' arena slot
  // tailored for `!Send` WASM values). The counter and "active"
  // flag are regular `RwSignal`s — they already carry the
  // `Send + Sync` bound through the reactive arena.
  let frame_counter = RwSignal::<u32>::new(0);
  let raf_active = RwSignal::<bool>::new(true);
  let raf_slot = StoredValue::new_local(Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>)));

  let raf_slot_inner = raf_slot;
  let raf_closure = Closure::wrap(Box::new(move |_ts: f64| {
    // `raf_active` flips to false on component cleanup and acts as a
    // one-way kill-switch. Check it up-front so a still-pending frame
    // callback that fires during unmount is a no-op.
    if !raf_active.get_untracked() {
      return;
    }
    frame_counter.update(|n| *n = n.saturating_add(1));
    // Re-arm: schedule the next animation frame. We use `try_borrow`
    // so a concurrent cleanup that is mutably borrowing the slot to
    // take the closure does not trip a runtime panic — it just
    // declines to reschedule, which is the desired behaviour.
    let cell = raf_slot_inner.get_value();
    if let Ok(borrowed) = cell.try_borrow()
      && let Some(closure) = borrowed.as_ref()
      && let Some(win) = web_sys::window()
      && let Err(err) =
        win.request_animation_frame(closure.as_ref().unchecked_ref::<js_sys::Function>())
    {
      web_sys::console::warn_1(&format!("[theater] RAF reschedule failed: {err:?}").into());
    }
  }) as Box<dyn FnMut(f64)>);

  // Install the first RAF — guarded in case the component mounts
  // outside a browser context (SSR / unit tests).
  if let Some(win) = web_sys::window()
    && let Err(err) =
      win.request_animation_frame(raf_closure.as_ref().unchecked_ref::<js_sys::Function>())
  {
    web_sys::console::warn_1(&format!("[theater] RAF initial schedule failed: {err:?}").into());
  }
  {
    let cell = raf_slot.get_value();
    *cell.borrow_mut() = Some(raf_closure);
  }

  // 1 Hz sampler — computes the drop rate for the previous second
  // and potentially triggers a degradation action.
  let drop_snapshot = RwSignal::new(crate::theater::FrameDropSnapshot::default());
  let drop_tick = set_interval_with_handle(
    move || {
      if state.my_role.get_untracked() != TheaterRole::Owner {
        // Reset the counter so a later owner promotion starts fresh.
        frame_counter.set(0);
        return;
      }
      let frames = frame_counter.get_untracked();
      frame_counter.set(0);
      let mut action = crate::theater::FrameDropAction::Hold;
      drop_snapshot.update(|snap| {
        action = crate::theater::evaluate_second(snap, frames, crate::theater::NOMINAL_FPS);
      });
      if matches!(action, crate::theater::FrameDropAction::Degrade) {
        let tier = state.quality_tier.get_untracked();
        state.quality_tier.set(crate::theater::degrade_tier(tier));
        state.owner_high_load.set(true);
      }
    },
    std::time::Duration::from_millis(1_000),
  );
  if let Ok(handle) = drop_tick {
    on_cleanup(move || {
      handle.clear();
    });
  }
  // Register RAF cleanup unconditionally — even when the 1 Hz sampler
  // failed to install, the RAF closure itself has already been
  // scheduled and must be dropped on unmount to release the browser
  // reference; otherwise a late frame callback would invoke a
  // now-dropped Rust closure (the "closure invoked recursively or
  // after being dropped" WASM error).
  on_cleanup(move || {
    raf_active.set(false);
    let cell = raf_slot.get_value();
    if let Ok(mut borrowed) = cell.try_borrow_mut() {
      borrowed.take();
    }
  });

  // ── Effect 4: auto-persist overlay settings on change ───────────────
  let persist_first_run = RwSignal::new(true);
  Effect::new(move |_| {
    // Subscribe to the overlay_settings signal; whenever it changes
    // (danmaku visibility, font size, subtitle appearance, etc.) we
    // persist to localStorage so the viewer's preferences survive
    // page refreshes.
    let _ = state.overlay_settings.get();
    // Skip the initial run — the settings are already loaded from
    // localStorage during TheaterState construction, so writing them
    // back immediately is a no-op that wastes a synchronous
    // localStorage call.
    if persist_first_run.get_untracked() {
      persist_first_run.set(false);
      return;
    }
    state.persist_overlay_settings();
  });

  // ── Effect 5: clean up TheaterState on unmount ─────────────────
  on_cleanup(move || {
    state.leave();
  });

  let room_name = move || state.room_name.get();
  let is_owner = move || state.my_role.get() == TheaterRole::Owner;

  view! {
    <section
      node_ref=section_ref
      class="theater-page"
      class:theater-page--fullscreen=move || state.is_fullscreen.get()
      data-testid="theater-page"
      aria-label=move || t_string!(i18n, theater.title)
    >
      <header class="theater-page__header">
        <h2 class="theater-page__title">{room_name}</h2>
      </header>
      <TheaterGraceBanner />
      <Show when=move || is_owner() && state.owner_high_load.get()>
        <aside
          class="theater-load-banner"
          role="status"
          aria-live="polite"
          data-testid="theater-load-banner"
        >
          <Icon icon=i::LuInfo attr:class="theater-load-banner__icon" />
          <span>{t!(i18n, theater.high_load_banner)}</span>
        </aside>
      </Show>

      <div class="theater-page__body">
        <div class="theater-page__stage">
          <CopyrightNotice />

          <div class="theater-page__surface">
            <TheaterVideoPlayer video_ref=video_ref />
            <SubtitleOverlay />
            <DanmakuCanvas />
            <Show when=move || state.is_fullscreen.get()>
              <button
                type="button"
                class="theater-page__panel-toggle"
                on:click=move |_| state.panel_visible.update(|v| *v = !*v)
                aria-label=move || t_string!(i18n, theater.show_panel).to_string()
                data-testid="theater-panel-toggle"
              >
                <Icon icon=i::LuMessageSquare />
              </button>
            </Show>
          </div>

          <TheaterPlaybackControls video_ref=video_ref fullscreen_target=section_ref />

          <div class="theater-page__composer">
            <DanmakuInput />
            <Show when=is_owner>
              <div class="theater-page__owner-panels">
                <SubtitleSettingsPanel />
                <DanmakuSettingsPanel />
              </div>
            </Show>
          </div>
        </div>

        <aside
          class="theater-page__side"
          class:theater-page__side--hidden=move || {
            state.is_fullscreen.get() && !state.panel_visible.get()
          }
        >
          <nav class="theater-page__tabs" role="tablist">
            <button
              type="button"
              class="theater-page__tab"
              class:is-active=move || active_tab.get() == SidePanelTab::Chat
              on:click=move |_| active_tab.set(SidePanelTab::Chat)
              role="tab"
              aria-selected=move || (active_tab.get() == SidePanelTab::Chat).to_string()
              data-testid="theater-tab-chat"
            >
              <Icon icon=i::LuMessageSquare />
              <span>{t!(i18n, theater.chat_title)}</span>
              <Show when=move || { state.chat_unread.get() > 0 }>
                <span class="theater-page__badge" aria-live="polite">
                  {move || state.chat_unread.get()}
                </span>
              </Show>
            </button>
            <button
              type="button"
              class="theater-page__tab"
              class:is-active=move || active_tab.get() == SidePanelTab::Members
              on:click=move |_| active_tab.set(SidePanelTab::Members)
              role="tab"
              aria-selected=move || (active_tab.get() == SidePanelTab::Members).to_string()
              data-testid="theater-tab-members"
            >
              <Icon icon=i::LuUsers />
              <span>{t!(i18n, theater.members_title)}</span>
            </button>
          </nav>

          <div class="theater-page__tab-panel">
            <Show
              when=move || active_tab.get() == SidePanelTab::Chat
              fallback=move || view! {
                <TheaterMemberPanel
                  room_id=Signal::derive(move || room.with(|r| r.room_id.clone()))
                />
              }
            >
              <TheaterChatPanel />
            </Show>
          </div>
        </aside>
      </div>
    </section>
  }
}
