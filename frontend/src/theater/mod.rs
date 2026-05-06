//! Theater Mode — shared video watching with danmaku, subtitles, and
//! real-time chat (Req 12).
//!
//! This module implements the full client-side theater experience:
//!
//! * Subtitle loading (SRT / WebVTT parsers, Req 12.4a)
//! * Danmaku dispatcher with 50 ms batch merge (Req 12.5 §28)
//! * Theater room state — role tracking, playback status, subtitle
//!   track, danmaku overlay toggle, viewer volume …
//! * Owner resource monitor (auto-degradation / auto-restore, Req 12.2)
//!
//! UI components live in `crate::components::theater` and consume the
//! state exposed here through Leptos signals.

pub mod chat_model;
pub mod danmaku;
pub mod danmaku_render;
pub mod dc_router;
pub mod frame_drop_monitor;
pub mod grace;
pub mod playback;
pub mod resource_monitor;
pub mod state;
pub mod subtitle;
pub mod subtitle_sync;

pub use chat_model::{
  CHAT_MESSAGE_HISTORY_CAP, RelativeTimeLabel, TheaterChatMessage, append_message,
  relative_time_label,
};
pub use danmaku::{DanmakuBatcher, DanmakuEntry};
pub use danmaku_render::{
  LANE_COOLDOWN_MS, LANE_COUNT, PINNED_DURATION_MS, RenderedDanmaku, build_rendered, color_to_css,
  font_size_px, is_expired, opacity_value, pick_lane, scroll_duration_ms, scroll_x_percent,
};
pub use dc_router::{
  TheaterInbound, apply as apply_theater_inbound, classify as classify_theater_inbound,
  should_dispatch as should_dispatch_theater_inbound,
};
pub use frame_drop_monitor::{
  DEGRADATION_HOLD_SECONDS as FRAME_DROP_HOLD_SECONDS,
  DROP_RATE_THRESHOLD_PERCENT as FRAME_DROP_THRESHOLD_PERCENT, FrameDropAction, FrameDropSnapshot,
  NOMINAL_FPS, drop_rate_percent, evaluate_second,
};
pub use grace::{GRACE_WINDOW_SECONDS, compute_grace_remaining, is_grace_expired};
pub use playback::{
  PROGRESS_BROADCAST_INTERVAL_MS, SEEK_TOLERANCE_MS, apply_playback_progress, build_progress_frame,
  format_timestamp, needs_seek, should_broadcast_progress,
};
pub use resource_monitor::{
  BANDWIDTH_HIGH_UTILIZATION_PERCENT, BandwidthEstimate, BandwidthSnapshot, DEFAULT_CAPACITY_BPS,
  DEGRADATION_HOLD_SECONDS, DEGRADATION_THRESHOLD_BYTES, MonitorAction, MonitorSnapshot,
  RECOVERY_HOLD_SECONDS, RECOVERY_THRESHOLD_BYTES, degrade_tier, evaluate_bandwidth, evaluate_tick,
  is_high_load, restore_tier,
};
pub use state::{
  PlaybackSnapshot, QualityTier, SharedBatcher, SubtitleAppearance, SubtitlePosition,
  SubtitleTrack, TheaterOverlaySettings, TheaterRole, TheaterState, provide_theater_state,
  use_theater_state,
};
pub use subtitle::{SubtitleParseError, parse_srt, parse_subtitle_file, parse_vtt};
pub use subtitle_sync::{
  apply_subtitle_clear, apply_subtitle_data, apply_subtitle_track, build_track_from_data,
  pick_active_text, refresh_active_subtitle, should_apply_clear,
};
