//! Network service layer (WebSocket client, WebRTC management)
//!
//! WebSocket is used only for signaling communication; all chat messages and
//! files are transferred through DataChannel P2P.

pub mod webrtc;
pub mod ws;
