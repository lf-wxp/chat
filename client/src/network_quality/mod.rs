//! Network quality monitoring module
//!
//! Collects network statistics periodically via `RTCPeerConnection.getStats()`,
//! determines network quality level based on RTT, packet loss, available bandwidth,
//! and automatically adjusts video encoding parameters to adapt to current network conditions.
//!
//! Architecture:
//! - `NetworkQualityManager`: Manages periodic collection tasks
//! - `NetworkStats`: Single collection statistics snapshot
//! - `QualityLevel`: Network quality level (Excellent / Good / Fair / Poor)
//! - Adaptive strategy: Dynamically adjust `RTCRtpSender.setParameters()` based on quality level

mod manager;
mod types;

// Re-export all public types
pub use manager::NetworkQualityManager;
pub use types::{
  AlertSeverity, AlertType, NetworkAlert, NetworkStats, PeerNetworkHistory, QualityLevel,
};

#[cfg(test)]
mod tests;
