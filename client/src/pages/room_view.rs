//! Room and Theater page views
//!
//! Integrates room member list, group chat, and theater components
//! into full page views with route parameter extraction.

use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_router::hooks::use_params_map;

use message::signal::{MemberRole, SignalMessage};

use crate::{
  components::{Avatar, AvatarSize, Button, ButtonVariant, EmptyState},
  i18n::*,
  services::ws::WsClient,
  state,
};

// =============================================================================
// RoomView
// =============================================================================

/// Room page — displays member sidebar + group chat area.
///
/// Route: `/room/:id`
#[component]
pub fn RoomView() -> impl IntoView {
  let params = use_params_map();
  let room_id = move || params.read().get("id").unwrap_or_default();

  let room_state = state::use_room_state();
  let user_state = state::use_user_state();
  let i18n = use_i18n();

  // Join room on mount
  Effect::new({
    let initial_id = room_id();
    move |_| {
      if !initial_id.is_empty() {
        let ws = WsClient::use_client();
        let _ = ws.send(&SignalMessage::JoinRoom {
          room_id: initial_id.clone(),
          password: None,
        });
      }
    }
  });

  // Leave room handler
  let handle_leave = move |()| {
    let rid = room_id();
    if !rid.is_empty() {
      let ws = WsClient::use_client();
      let _ = ws.send(&SignalMessage::LeaveRoom { room_id: rid });
    }
    // Navigate back
    if let Some(window) = web_sys::window() {
      let _ = window.location().set_href("/");
    }
  };

  view! {
    <div class="page-room">
      <div class="room-layout">
        // Member sidebar
        <aside class="room-sidebar">
          <div class="room-sidebar-header">
          <h3 class="room-sidebar-title">{t_string!(i18n, room_members_title)}</h3>
            <span class="room-member-count">
              {move || {
                let members = &room_state.get().current_room_members;
                let len = members.len();
                format!("({len})")
              }}
            </span>
          </div>
          <div class="room-member-list">
            {move || {
              let members = room_state.get().current_room_members.clone();
              let my_id = user_state.get_untracked().user_id.clone();

              // Find my role
              let my_role = members.iter()
                .find(|m| m.user_id == my_id)
                .map_or(MemberRole::Member, |m| m.role);

              members.into_iter().map(|member| {
                let member_id = member.user_id.clone();
                let member_id_kick = member.user_id.clone();
                let member_id_mute = member.user_id.clone();
                let is_me = member_id == my_id;
                let role_badge = match member.role {
                  MemberRole::Owner => "👑",
                  MemberRole::Viewer => "👁️",
                  MemberRole::Member => "",
                };
                let is_muted = member.muted;
                let can_manage = matches!(my_role, MemberRole::Owner) && !is_me;

                view! {
                  <div class="room-member-item">
                    <Avatar username=member_id.clone() size=AvatarSize::Small online=true />
                    <div class="room-member-info">
                      <span class="room-member-name">
                        {member_id.clone()}
                        {if role_badge.is_empty() {
                          String::new()
                        } else {
                          format!(" {role_badge}")
                        }}
                      </span>
                      {if is_muted {
                        view! { <span class="room-member-muted">"🔇"</span> }.into_any()
                      } else {
                        view! { <span></span> }.into_any()
                      }}
                    </div>
                    {if can_manage {
                      let rid_kick = room_id();
                      let rid_mute = room_id();
                      view! {
                        <div class="room-member-actions">
                          <button
                            class="tool-btn tool-btn-sm"
                            tabindex=0
                            aria-label=t_string!(i18n, room_kick_member)
                            title=t_string!(i18n, room_kick)
                            on:click=move |_| {
                              let ws = WsClient::use_client();
                              let _ = ws.send(&SignalMessage::KickMember {
                                room_id: rid_kick.clone(),
                                target_user_id: member_id_kick.clone(),
                              });
                            }
                          >"🚫"</button>
                          <button
                            class="tool-btn tool-btn-sm"
                            tabindex=0
                            aria-label=if is_muted { t_string!(i18n, room_unmute_member) } else { t_string!(i18n, room_mute_member) }
                            title=if is_muted { t_string!(i18n, room_unmute_member) } else { t_string!(i18n, room_mute_member) }
                            on:click=move |_| {
                              let ws = WsClient::use_client();
                              let _ = ws.send(&SignalMessage::MuteMember {
                                room_id: rid_mute.clone(),
                                target_user_id: member_id_mute.clone(),
                                muted: !is_muted,
                              });
                            }
                          >{if is_muted { "🔊" } else { "🔇" }}</button>
                        </div>
                      }.into_any()
                    } else {
                      view! { <span></span> }.into_any()
                    }}
                  </div>
                }
              }).collect_view()
            }}
          </div>
          <div class="room-sidebar-footer">
            <Button
              label=t_string!(i18n, room_leave).to_string()
              variant=ButtonVariant::Ghost
              on_click=Callback::new(handle_leave)
            />
          </div>
        </aside>

        // Main chat area
        <main class="room-main">
          <div class="room-header">
            <h2 class="room-title">
              {move || {
                let rid = room_id();
                let rooms = &room_state.get().rooms;
                rooms.iter()
                  .find(|r| r.room_id == rid)
                  .map_or_else(|| {
                    let fallback = t_string!(i18n, room_fallback_name).to_string().replace("{}", &rid);
                    fallback
                  }, |r| r.name.clone())
              }}
            </h2>
          </div>
          <div class="room-chat-area">
            <EmptyState
              icon="💬"
              title=t_string!(i18n, room_chat).to_string()
              description=""
            />
          </div>
        </main>
      </div>
    </div>
  }
}

// =============================================================================
// TheaterView
// =============================================================================

/// Theater page — integrates the TheaterPanel component.
///
/// Route: `/theater/:id`
#[component]
pub fn TheaterView() -> impl IntoView {
  let params = use_params_map();
  let room_id = move || params.read().get("id").unwrap_or_default();

  let theater_state = state::use_theater_state();
  let i18n = use_i18n();

  // Initialize theater state with room_id on mount
  Effect::new({
    let initial_id = room_id();
    move |_| {
      if !initial_id.is_empty() {
        theater_state.update(|s| {
          s.theater_id = Some(initial_id.clone());
        });
        // Join the theater room
        let ws = WsClient::use_client();
        let _ = ws.send(&SignalMessage::JoinRoom {
          room_id: initial_id.clone(),
          password: None,
        });
      }
    }
  });

  // Leave handler
  let handle_leave = move || {
    let rid = room_id();
    if !rid.is_empty() {
      let ws = WsClient::use_client();
      let _ = ws.send(&SignalMessage::LeaveRoom { room_id: rid });
      theater_state.update(|s| {
        s.theater_id = None;
        s.video_url = None;
        s.is_playing = false;
        s.current_time = 0.0;
        s.danmaku_list.clear();
      });
    }
    if let Some(window) = web_sys::window() {
      let _ = window.location().set_href("/");
    }
  };

  view! {
    <div class="page-theater">
      {move || {
        let rid = room_id();
        if rid.is_empty() {
          view! {
            <EmptyState
              icon="🎬"
              title=t_string!(i18n, nav_theater).to_string()
              description=""
            />
          }.into_any()
        } else {
          view! {
            <div class="theater-layout">
              <div class="theater-content">
                <crate::theater::TheaterPanel room_id=rid.clone() />
              </div>
              <div class="theater-footer">
                <Button
                  label=t_string!(i18n, theater_leave).to_string()
                  variant=ButtonVariant::Ghost
                  on_click=Callback::new(move |()| handle_leave())
                />
              </div>
            </div>
          }.into_any()
        }
      }}
    </div>
  }
}
