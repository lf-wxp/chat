//! Home page component.
//!
//! Renders the active chat or theater view. When no conversation is
//! selected, a welcoming empty state is shown prompting the user to
//! select a conversation from the sidebar or join a room.

use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;
use message::types::RoomType;

use crate::components::chat_view::ChatView;
use crate::components::theater::TheaterPage;
use crate::i18n;
use crate::state::{ConversationId, use_app_state};

/// Home page component.
#[component]
pub fn HomePage() -> impl IntoView {
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();

  let has_conversation = Memo::new(move |_| app_state.active_conversation.get().is_some());

  // Resolve the current theater RoomInfo (if any). `None` means the
  // active conversation is either a direct message or a plain chat
  // room, in which case `<ChatView/>` handles the rendering.
  let theater_room = Memo::new(move |_| {
    let conv = app_state.active_conversation.get()?;
    let ConversationId::Room(rid) = conv else {
      return None;
    };
    app_state.rooms.with(|rooms| {
      rooms
        .iter()
        .find(|r| r.room_id == rid && r.room_type == RoomType::Theater)
        .cloned()
    })
  });

  view! {
    <Show when=move || has_conversation.get()>
      <Show
        when=move || theater_room.get().is_some()
        fallback=move || view! { <ChatView /> }
      >
        {move || theater_room.get().map(|info| {
          let info_signal = Signal::derive(move || info.clone());
          view! { <TheaterPage room=info_signal /> }
        })}
      </Show>
    </Show>
    <Show when=move || !has_conversation.get()>
      <div class="home-empty" data-testid="home-empty">
        <div class="home-empty__icon"><Icon icon=i::LuMessageSquare /></div>
        <h2 class="home-empty__title">{t_string!(i18n, app.title)}</h2>
        <p class="home-empty__hint">{t_string!(i18n, home.select_conversation)}</p>
      </div>
    </Show>
  }
}
