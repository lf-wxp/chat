//! Error code constants for all modules.
//!
//! Each constant follows the naming convention: `{MODULE}{CATEGORY}{SEQUENCE}`
//! - Module prefix: SIG, CHT, AV, ROM, THR, FIL, AUTH, PST, SYS
//! - Category: 0=Network, 1=Client, 2=Informational, 3=Server, 4=Media, 5=Security
//! - Sequence: 2-digit number within module-category

use super::{ErrorCategory, ErrorCode, ErrorModule};

// ============================================================================
// Error Code Constants - Signaling (SIG)
// ============================================================================

/// WebSocket connection failed
pub const SIG001: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 1);
/// SDP negotiation timeout
pub const SIG002: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2);
/// ICE connection failed
pub const SIG003: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 3);
/// Heartbeat timeout
pub const SIG004: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 4);
/// WebSocket reconnection failed
pub const SIG005: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 5);
/// Invalid SDP format
pub const SIG101: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Client, 1);
/// Invalid ICE candidate
pub const SIG102: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Client, 2);
/// Invalid message type
pub const SIG103: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Client, 3);
/// Rate limit exceeded
pub const SIG104: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Client, 4);

// ============================================================================
// Error Code Constants - Chat (CHT)
// ============================================================================

/// `DataChannel` send failed
pub const CHT001: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Network, 1);
/// `DataChannel` receive failed
pub const CHT002: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Network, 2);
/// Message ACK timeout
pub const CHT003: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Network, 3);
/// Message too long (max 10000 chars)
pub const CHT101: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Client, 1);
/// Invalid sticker ID
pub const CHT102: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Client, 2);
/// Message revoke timeout (2 minutes exceeded)
pub const CHT103: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Client, 3);
/// Message already revoked
pub const CHT104: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Client, 4);
/// Empty message not allowed
pub const CHT105: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Client, 5);
/// Encryption failed
pub const CHT501: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Security, 1);
/// Decryption failed
pub const CHT502: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Security, 2);

// ============================================================================
// Error Code Constants - Audio/Video (AV)
// ============================================================================

/// `PeerConnection` disconnected during call
pub const AV001: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Network, 1);
/// Media connection timeout
pub const AV002: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Network, 2);
/// Camera access denied
pub const AV401: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Media, 1);
/// Microphone access denied
pub const AV402: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Media, 2);
/// Screen share cancelled
pub const AV403: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Media, 3);
/// Screen share denied
pub const AV404: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Media, 4);
/// Codec not supported
pub const AV405: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Media, 5);

// ============================================================================
// Error Code Constants - Room (ROM)
// ============================================================================

/// Room join timeout
pub const ROM001: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Network, 1);
/// Room leave timeout
pub const ROM002: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Network, 2);
/// Room password incorrect
pub const ROM101: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 1);
/// Room is full (max 8 members)
pub const ROM102: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 2);
/// Insufficient permissions
pub const ROM103: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 3);
/// User already in room
pub const ROM104: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 4);
/// Room not found
pub const ROM105: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 5);
/// User banned from room
pub const ROM106: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 6);
/// User muted in room
pub const ROM107: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 7);
/// Cannot kick/modify owner
pub const ROM108: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 8);

// ============================================================================
// Error Code Constants - Theater (THR)
// ============================================================================

/// Theater video stream failed
pub const THR001: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Network, 1);
/// Theater sync timeout
pub const THR002: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Network, 2);
/// Theater owner disconnected
pub const THR003: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Network, 3);
/// Not theater owner
pub const THR101: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Client, 1);
/// Invalid video source
pub const THR102: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Client, 2);
/// Danmaku too long (max 100 chars)
pub const THR103: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Client, 3);
/// Subtitle format invalid
pub const THR104: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Client, 4);

// ============================================================================
// Error Code Constants - File Transfer (FIL)
// ============================================================================

/// File transfer interrupted
pub const FIL001: ErrorCode = ErrorCode::new(ErrorModule::Fil, ErrorCategory::Network, 1);
/// File chunk timeout
pub const FIL002: ErrorCode = ErrorCode::new(ErrorModule::Fil, ErrorCategory::Network, 2);
/// File too large (single: 100MB, multi: 20MB)
pub const FIL101: ErrorCode = ErrorCode::new(ErrorModule::Fil, ErrorCategory::Client, 1);
/// File type not allowed
pub const FIL102: ErrorCode = ErrorCode::new(ErrorModule::Fil, ErrorCategory::Client, 2);
/// File hash mismatch
pub const FIL103: ErrorCode = ErrorCode::new(ErrorModule::Fil, ErrorCategory::Client, 3);
/// Dangerous file extension warning
pub const FIL104: ErrorCode = ErrorCode::new(ErrorModule::Fil, ErrorCategory::Client, 4);

// ============================================================================
// Error Code Constants - Authentication (AUTH)
// ============================================================================

/// Authentication timeout
pub const AUTH001: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Network, 1);
/// JWT token expired
pub const AUTH501: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Security, 1);
/// JWT token invalid
pub const AUTH502: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Security, 2);
/// Session invalidated (another device logged in)
pub const AUTH503: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Security, 3);
/// Invalid credentials
pub const AUTH101: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Client, 1);
/// User already exists
pub const AUTH102: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Client, 2);
/// User not found
pub const AUTH103: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Client, 3);

// ============================================================================
// Error Code Constants - Persistence (PST)
// ============================================================================

/// `IndexedDB` write failed
pub const PST301: ErrorCode = ErrorCode::new(ErrorModule::Pst, ErrorCategory::Server, 1);
/// `IndexedDB` read failed
pub const PST302: ErrorCode = ErrorCode::new(ErrorModule::Pst, ErrorCategory::Server, 2);
/// Storage quota exceeded
pub const PST303: ErrorCode = ErrorCode::new(ErrorModule::Pst, ErrorCategory::Server, 3);

// ============================================================================
// Error Code Constants - System (SYS)
// ============================================================================

/// Browser offline
pub const SYS001: ErrorCode = ErrorCode::new(ErrorModule::Sys, ErrorCategory::Network, 1);
/// WebRTC not supported
pub const SYS101: ErrorCode = ErrorCode::new(ErrorModule::Sys, ErrorCategory::Client, 1);
/// WebSocket not supported
pub const SYS102: ErrorCode = ErrorCode::new(ErrorModule::Sys, ErrorCategory::Client, 2);
/// `IndexedDB` not supported
pub const SYS103: ErrorCode = ErrorCode::new(ErrorModule::Sys, ErrorCategory::Client, 3);
/// Browser not supported
pub const SYS301: ErrorCode = ErrorCode::new(ErrorModule::Sys, ErrorCategory::Server, 1);

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests;
