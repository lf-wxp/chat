//! Theater-mode UI components (Req 12).
//!
//! Houses the presentation layer for the Theater feature (copyright
//! notice, video player, playback controls, subtitle overlay, danmaku
//! canvas, member list, theater page …). Each component consumes the
//! reactive state exposed by `crate::theater::TheaterState`.

mod copyright_notice;
mod danmaku_canvas;
mod danmaku_input;
mod danmaku_item;
mod danmaku_settings_panel;
mod subtitle_overlay;
mod subtitle_settings_panel;
mod theater_chat_bubble;
mod theater_chat_panel;
mod theater_grace_banner;
mod theater_member_panel;
mod theater_page;
mod theater_playback_controls;
mod theater_video_player;
mod video_source_picker;

pub use copyright_notice::CopyrightNotice;
pub use danmaku_canvas::DanmakuCanvas;
pub use danmaku_input::DanmakuInput;
pub use danmaku_item::DanmakuItem;
pub use danmaku_settings_panel::DanmakuSettingsPanel;
pub use subtitle_overlay::SubtitleOverlay;
pub use subtitle_settings_panel::SubtitleSettingsPanel;
pub use theater_chat_bubble::TheaterChatBubble;
pub use theater_chat_panel::TheaterChatPanel;
pub use theater_grace_banner::TheaterGraceBanner;
pub use theater_member_panel::TheaterMemberPanel;
pub use theater_page::TheaterPage;
pub use theater_playback_controls::TheaterPlaybackControls;
pub use theater_video_player::TheaterVideoPlayer;
pub use video_source_picker::{VideoSource, VideoSourceKind, VideoSourcePicker};
