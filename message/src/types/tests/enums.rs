//! Enum type tests: display, default, boundary, and edge case tests.

use super::*;

// ===========================================================================
// RoomType tests
// ===========================================================================

#[test]
fn test_room_type_chat_display() {
  assert_eq!(format!("{}", RoomType::Chat), "Chat");
}

#[test]
fn test_room_type_theater_display() {
  assert_eq!(format!("{}", RoomType::Theater), "Theater");
}

#[test]
fn test_room_type_default() {
  let default = RoomType::default();
  assert_eq!(default, RoomType::Chat);
}

// ===========================================================================
// UserStatus tests
// ===========================================================================

#[test]
fn test_user_status_online_display() {
  assert_eq!(format!("{}", UserStatus::Online), "Online");
}

#[test]
fn test_user_status_offline_display() {
  assert_eq!(format!("{}", UserStatus::Offline), "Offline");
}

#[test]
fn test_user_status_away_display() {
  assert_eq!(format!("{}", UserStatus::Away), "Away");
}

#[test]
fn test_user_status_busy_display() {
  assert_eq!(format!("{}", UserStatus::Busy), "Busy");
}

#[test]
fn test_user_status_default() {
  let default = UserStatus::default();
  assert_eq!(default, UserStatus::Online);
}

// ===========================================================================
// NetworkQuality tests
// ===========================================================================

#[test]
fn test_network_quality_excellent_display() {
  assert_eq!(format!("{}", NetworkQuality::Excellent), "Excellent");
}

#[test]
fn test_network_quality_good_display() {
  assert_eq!(format!("{}", NetworkQuality::Good), "Good");
}

#[test]
fn test_network_quality_fair_display() {
  assert_eq!(format!("{}", NetworkQuality::Fair), "Fair");
}

#[test]
fn test_network_quality_poor_display() {
  assert_eq!(format!("{}", NetworkQuality::Poor), "Poor");
}

#[test]
fn test_network_quality_default() {
  let default = NetworkQuality::default();
  assert_eq!(default, NetworkQuality::Good);
}

#[test]
fn test_network_quality_from_metrics_excellent() {
  let quality = NetworkQuality::from_metrics(50, 0.5);
  assert_eq!(quality, NetworkQuality::Excellent);
}

#[test]
fn test_network_quality_from_metrics_good() {
  let quality = NetworkQuality::from_metrics(150, 2.0);
  assert_eq!(quality, NetworkQuality::Good);
}

#[test]
fn test_network_quality_from_metrics_fair() {
  let quality = NetworkQuality::from_metrics(300, 5.0);
  assert_eq!(quality, NetworkQuality::Fair);
}

#[test]
fn test_network_quality_from_metrics_poor() {
  let quality = NetworkQuality::from_metrics(500, 15.0);
  assert_eq!(quality, NetworkQuality::Poor);
}

#[test]
fn test_network_quality_recommended_video_quality() {
  assert_eq!(
    NetworkQuality::Excellent.recommended_video_quality(),
    "1080p"
  );
  assert_eq!(NetworkQuality::Good.recommended_video_quality(), "720p");
  assert_eq!(NetworkQuality::Fair.recommended_video_quality(), "480p");
  assert_eq!(NetworkQuality::Poor.recommended_video_quality(), "360p");
}

// ===========================================================================
// MessageContentType tests
// ===========================================================================

#[test]
fn test_message_content_type_text_display() {
  assert_eq!(format!("{}", MessageContentType::Text), "Text");
}

#[test]
fn test_message_content_type_image_display() {
  assert_eq!(format!("{}", MessageContentType::Image), "Image");
}

#[test]
fn test_message_content_type_system_display() {
  assert_eq!(format!("{}", MessageContentType::System), "System");
}

#[test]
fn test_message_content_type_sticker_display() {
  assert_eq!(format!("{}", MessageContentType::Sticker), "Sticker");
}

#[test]
fn test_message_content_type_voice_display() {
  assert_eq!(format!("{}", MessageContentType::Voice), "Voice");
}

#[test]
fn test_message_content_type_file_display() {
  assert_eq!(format!("{}", MessageContentType::File), "File");
}

#[test]
fn test_message_content_type_default() {
  let default = MessageContentType::default();
  assert_eq!(default, MessageContentType::Text);
}

// ===========================================================================
// MediaType tests
// ===========================================================================

#[test]
fn test_media_type_audio_display() {
  assert_eq!(format!("{}", MediaType::Audio), "Audio");
}

#[test]
fn test_media_type_video_display() {
  assert_eq!(format!("{}", MediaType::Video), "Video");
}

#[test]
fn test_media_type_screen_share_display() {
  assert_eq!(format!("{}", MediaType::ScreenShare), "Screen Share");
}

#[test]
fn test_media_type_default() {
  let default = MediaType::default();
  assert_eq!(default, MediaType::Audio);
}

// ===========================================================================
// DanmakuPosition tests
// ===========================================================================

#[test]
fn test_danmaku_position_variants() {
  // DanmakuPosition does not implement Display; test variant equality
  assert_eq!(DanmakuPosition::Scroll, DanmakuPosition::Scroll);
  assert_eq!(DanmakuPosition::Top, DanmakuPosition::Top);
  assert_eq!(DanmakuPosition::Bottom, DanmakuPosition::Bottom);
}

#[test]
fn test_danmaku_position_default() {
  let default = DanmakuPosition::default();
  assert_eq!(default, DanmakuPosition::Scroll);
}

#[test]
fn test_danmaku_position_debug() {
  let scroll = DanmakuPosition::Scroll;
  let debug_str = format!("{scroll:?}");
  assert!(!debug_str.is_empty());
}

// ===========================================================================
// ReactionAction tests
// ===========================================================================

#[test]
fn test_reaction_action_variants() {
  // ReactionAction does not implement Display; test variant equality
  assert_eq!(ReactionAction::Add, ReactionAction::Add);
  assert_eq!(ReactionAction::Remove, ReactionAction::Remove);
}

#[test]
fn test_reaction_action_default() {
  let default = ReactionAction::default();
  assert_eq!(default, ReactionAction::Add);
}

#[test]
fn test_reaction_action_debug() {
  let add = ReactionAction::Add;
  let debug_str = format!("{add:?}");
  assert!(!debug_str.is_empty());
}

// ===========================================================================
// RoomRole tests
// ===========================================================================

#[test]
fn test_room_role_variants() {
  assert_eq!(RoomRole::Owner, RoomRole::Owner);
  assert_eq!(RoomRole::Admin, RoomRole::Admin);
  assert_eq!(RoomRole::Member, RoomRole::Member);
}

#[test]
fn test_room_role_default() {
  let default = RoomRole::default();
  assert_eq!(default, RoomRole::Member);
}
