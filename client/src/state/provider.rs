//! Global state provider and convenience accessor functions

use leptos::prelude::*;

use super::{
  ChatState, ConnectionState, NetworkQualityState, OnlineUsersState, RoomState, SearchState,
  ThemeState, TheaterState, UiState, UserState, VadState,
};

/// Initialize and provide all global states to the component tree
pub fn provide_app_state() {
  provide_context(RwSignal::new(UserState::default()));
  provide_context(RwSignal::new(ChatState::default()));
  provide_context(RwSignal::new(OnlineUsersState::default()));
  provide_context(RwSignal::new(RoomState::default()));
  provide_context(RwSignal::new(ThemeState::default()));
  provide_context(RwSignal::new(TheaterState::default()));
  provide_context(RwSignal::new(ConnectionState::default()));
  provide_context(RwSignal::new(UiState::default()));
  provide_context(RwSignal::new(VadState::default()));
  provide_context(RwSignal::new(NetworkQualityState::default()));
  provide_context(RwSignal::new(SearchState::default()));
}

/// Convenience function: get user state
pub fn use_user_state() -> RwSignal<UserState> {
  use_context::<RwSignal<UserState>>().expect("UserState not provided")
}

/// Convenience function: get chat state
pub fn use_chat_state() -> RwSignal<ChatState> {
  use_context::<RwSignal<ChatState>>().expect("ChatState not provided")
}

/// Convenience function: get online users state
pub fn use_online_users_state() -> RwSignal<OnlineUsersState> {
  use_context::<RwSignal<OnlineUsersState>>().expect("OnlineUsersState not provided")
}

/// Convenience function: get room state
pub fn use_room_state() -> RwSignal<RoomState> {
  use_context::<RwSignal<RoomState>>().expect("RoomState not provided")
}

/// Convenience function: get theme state
pub fn use_theme_state() -> RwSignal<ThemeState> {
  use_context::<RwSignal<ThemeState>>().expect("ThemeState not provided")
}

/// Convenience function: get theater state
pub fn use_theater_state() -> RwSignal<TheaterState> {
  use_context::<RwSignal<TheaterState>>().expect("TheaterState not provided")
}

/// Convenience function: get connection state
pub fn use_connection_state() -> RwSignal<ConnectionState> {
  use_context::<RwSignal<ConnectionState>>().expect("ConnectionState not provided")
}

/// Convenience function: get UI state
pub fn use_ui_state() -> RwSignal<UiState> {
  use_context::<RwSignal<UiState>>().expect("UiState not provided")
}

/// Convenience function: get VAD speaker detection state
pub fn use_vad_state() -> RwSignal<VadState> {
  use_context::<RwSignal<VadState>>().expect("VadState not provided")
}

/// Convenience function: get network quality state
pub fn use_network_quality_state() -> RwSignal<NetworkQualityState> {
  use_context::<RwSignal<NetworkQualityState>>().expect("NetworkQualityState not provided")
}

/// Convenience function: get message search state
pub fn use_search_state() -> RwSignal<SearchState> {
  use_context::<RwSignal<SearchState>>().expect("SearchState not provided")
}
