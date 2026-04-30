//! Collapsible announcement panel rendered at the top of a room's
//! chat view (Req 15.2 §11).

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::UserId;
use message::types::{RoomId, RoomRole};

use crate::components::room::announcement_editor::AnnouncementEditor;
use crate::components::room::utils::current_role;
use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::signaling::use_signaling_client;
use crate::state::use_app_state;
use icondata as i;
use leptos_icons::Icon;

/// Announcement panel.
#[component]
pub fn AnnouncementPanel(
  /// Active room id.
  #[prop(into)]
  room_id: Signal<RoomId>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();

  let collapsed = RwSignal::new(false);
  let editor_open = RwSignal::new(false);

  let announcement = Memo::new(move |_| {
    let rid = room_id.get();
    app_state.rooms.with(|list| {
      list
        .iter()
        .find(|r| r.room_id == rid)
        .map(|r| r.announcement.clone())
        .unwrap_or_default()
    })
  });

  let actor_id = Signal::derive(move || {
    app_state
      .auth
      .with(|a| a.as_ref().map(|a| a.user_id.clone()))
  });

  let actor_role = Memo::new(move |_| {
    let rid = room_id.get();
    app_state.room_members.with(|map| {
      let members = map.get(&rid);
      actor_id.get().map_or(RoomRole::Member, |id: UserId| {
        members.map_or(RoomRole::Member, |list| current_role(list, &id))
      })
    })
  });

  let can_edit = Memo::new(move |_| actor_role.get() == RoomRole::Owner);

  let signaling_for_save = signaling.clone();
  let toast_for_save = toast;
  let on_save = Callback::new(move |content: String| {
    let rid = room_id.get();
    if let Err(e) = signaling_for_save.send_room_announcement(rid, content) {
      web_sys::console::warn_1(&format!("[room] Failed to update announcement: {e}").into());
      toast_for_save.show_error_message_with_key(
        "ROM113",
        "error.rom113",
        t_string!(i18n, error.rom113),
      );
    }
    editor_open.set(false);
  });

  let on_cancel = Callback::new(move |()| editor_open.set(false));

  view! {
    // Full panel: only shown when there is actual content. This
    // satisfies Req 15.2 §15 ("hide the announcement panel completely
    // (no empty placeholder)").
    <Show when=move || !announcement.get().is_empty()>
      <section
        class="room-announcement-panel"
        class:room-announcement-panel--collapsed=move || collapsed.get()
        aria-label=move || t_string!(i18n, room.announcement)
        data-testid="room-announcement-panel"
      >
        <header class="room-announcement-panel__header">
          <button
            type="button"
            class="room-announcement-panel__toggle"
            aria-expanded=move || (!collapsed.get()).to_string()
            on:click=move |_| collapsed.update(|v| *v = !*v)
            data-testid="room-announcement-toggle"
          >
            <span class="room-announcement-panel__chevron" aria-hidden="true">
              {move || if collapsed.get() {
                view! { <Icon icon=i::LuChevronRight /> }.into_any()
              } else {
                view! { <Icon icon=i::LuChevronDown /> }.into_any()
              }}
            </span>
            <h3 class="room-announcement-panel__title">{t!(i18n, room.announcement)}</h3>
          </button>
          <Show when=move || can_edit.get()>
            <button
              type="button"
              class="btn btn--ghost room-announcement-panel__edit"
              on:click=move |_| editor_open.set(true)
              data-testid="room-announcement-edit"
            >
              {t!(i18n, common.edit)}
            </button>
          </Show>
        </header>
        <Show when=move || !collapsed.get()>
          <div class="room-announcement-panel__body">
            // SECURITY: inner_html is safe here because
            // render_preview_html HTML-escapes all raw text before
            // substituting Markdown tokens and only allows http(s)://
            // links. Any change to that function must be audited for
            // XSS regressions.
            <div
              class="room-announcement-panel__content"
              inner_html=move || {
                crate::components::room::announcement_editor::render_preview_html(
                  &announcement.get(),
                )
              }
            />
          </div>
        </Show>
      </section>
    </Show>

    // Empty-state CTA: shown ONLY to the owner when there is no
    // announcement yet. Renders a compact action button — NOT a panel —
    // so Req 15.2 §15 ("hide the announcement panel completely, no
    // empty placeholder") is fully satisfied. Non-owners see nothing
    // at all when there is no announcement.
    <Show when=move || announcement.get().is_empty() && can_edit.get()>
      <div class="room-announcement-panel__cta" data-testid="room-announcement-cta">
        <button
          type="button"
          class="btn btn--ghost room-announcement-panel__cta-btn"
          on:click=move |_| editor_open.set(true)
          data-testid="room-announcement-create"
        >
          {t!(i18n, room.announcement_create)}
        </button>
      </div>
    </Show>

    <Show when=move || editor_open.get()>
      <AnnouncementEditor
        initial=Signal::derive(move || announcement.get())
        on_save=on_save
        on_cancel=on_cancel
      />
    </Show>
  }
}
