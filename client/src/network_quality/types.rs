//! Network quality type definitions
//!
//! Contains all data structures for network quality monitoring.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

// =============================================================================
// Configuration Constants
// =============================================================================

/// Maximum number of historical data points to store per peer
pub const MAX_HISTORY_SIZE: usize = 60;

/// Alert thresholds for network quality degradation
#[derive(Clone)]
pub struct AlertThresholds {
  /// RTT threshold for warning (ms)
  pub rtt_warning_ms: f64,
  /// RTT threshold for critical (ms)
  pub rtt_critical_ms: f64,
  /// Packet loss threshold for warning (0.0-1.0)
  pub packet_loss_warning: f64,
  /// Packet loss threshold for critical (0.0-1.0)
  pub packet_loss_critical: f64,
  /// Jitter threshold for warning (ms)
  pub jitter_warning_ms: f64,
  /// Jitter threshold for critical (ms)
  pub jitter_critical_ms: f64,
  /// Consecutive poor quality readings before alert
  pub consecutive_poor_threshold: u32,
}

impl Default for AlertThresholds {
  fn default() -> Self {
    Self {
      rtt_warning_ms: 150.0,
      rtt_critical_ms: 300.0,
      packet_loss_warning: 0.03,
      packet_loss_critical: 0.08,
      jitter_warning_ms: 30.0,
      jitter_critical_ms: 50.0,
      consecutive_poor_threshold: 3,
    }
  }
}

// =============================================================================
// Network Quality Level
// =============================================================================

/// Network quality level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum QualityLevel {
  /// Excellent (RTT < 50ms, packet loss < 1%)
  Excellent,
  /// Good (RTT < 150ms, packet loss < 3%)
  #[default]
  Good,
  /// Fair (RTT < 300ms, packet loss < 8%)
  Fair,
  /// Poor (RTT >= 300ms or packet loss >= 8%)
  Poor,
}

impl QualityLevel {
  /// Determine quality level based on RTT (ms) and packet loss (0.0 ~ 1.0)
  #[must_use]
  pub fn from_stats(rtt_ms: f64, packet_loss: f64) -> Self {
    if rtt_ms < 50.0 && packet_loss < 0.01 {
      Self::Excellent
    } else if rtt_ms < 150.0 && packet_loss < 0.03 {
      Self::Good
    } else if rtt_ms < 300.0 && packet_loss < 0.08 {
      Self::Fair
    } else {
      Self::Poor
    }
  }

  /// Emoji icon for quality level
  #[must_use]
  pub fn icon(self) -> &'static str {
    let _ = self;
    "📶"
  }

  /// Text label for quality level
  #[must_use]
  pub fn label(self) -> &'static str {
    match self {
      Self::Excellent => "Excellent",
      Self::Good => "Good",
      Self::Fair => "Fair",
      Self::Poor => "Poor",
    }
  }

  /// CSS class suffix for quality level
  #[must_use]
  pub fn css_class(self) -> &'static str {
    match self {
      Self::Excellent => "excellent",
      Self::Good => "good",
      Self::Fair => "fair",
      Self::Poor => "poor",
    }
  }

  /// Color for quality level (hex format)
  #[must_use]
  pub fn color(self) -> &'static str {
    match self {
      Self::Excellent => "#22c55e",
      Self::Good => "#84cc16",
      Self::Fair => "#eab308",
      Self::Poor => "#ef4444",
    }
  }

  /// Recommended maximum video bitrate for this level (bps)
  pub fn target_max_bitrate(self) -> u32 {
    match self {
      Self::Excellent => 2_500_000,
      Self::Good => 1_500_000,
      Self::Fair => 800_000,
      Self::Poor => 300_000,
    }
  }

  /// Recommended maximum framerate for this level
  pub fn target_max_framerate(self) -> u32 {
    match self {
      Self::Excellent => 30,
      Self::Good => 24,
      Self::Fair => 15,
      Self::Poor => 10,
    }
  }

  /// Recommended scale resolution down factor for this level (1.0 = original)
  pub fn target_scale_resolution_down_by(self) -> f64 {
    match self {
      Self::Excellent | Self::Good => 1.0,
      Self::Fair => 1.5,
      Self::Poor => 2.0,
    }
  }
}

// =============================================================================
// Network Statistics Snapshot
// =============================================================================

/// Single collection of network statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkStats {
  /// Round-trip time (ms)
  pub rtt_ms: f64,
  /// Packet loss rate (0.0 ~ 1.0)
  pub packet_loss: f64,
  /// Available outgoing bandwidth (bps)
  pub available_outgoing_bitrate: f64,
  /// Current sending bitrate (bps)
  pub current_bitrate: f64,
  /// Jitter (ms)
  pub jitter_ms: f64,
  /// Quality level
  pub quality: QualityLevel,
  /// Timestamp of this measurement (ms since epoch)
  pub timestamp: f64,
}

// =============================================================================
// Historical Data Point
// =============================================================================

/// Historical data point for charting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryDataPoint {
  /// Timestamp (ms since epoch)
  pub timestamp: f64,
  /// RTT (ms)
  pub rtt_ms: f64,
  /// Packet loss (0.0-1.0, stored as percentage for display)
  pub packet_loss_percent: f64,
  /// Bitrate (kbps)
  pub bitrate_kbps: f64,
  /// Jitter (ms)
  pub jitter_ms: f64,
  /// Quality level
  pub quality: QualityLevel,
}

impl From<&NetworkStats> for HistoryDataPoint {
  fn from(stats: &NetworkStats) -> Self {
    Self {
      timestamp: stats.timestamp,
      rtt_ms: stats.rtt_ms,
      packet_loss_percent: stats.packet_loss * 100.0,
      bitrate_kbps: stats.current_bitrate / 1000.0,
      jitter_ms: stats.jitter_ms,
      quality: stats.quality,
    }
  }
}

// =============================================================================
// Network Alert
// =============================================================================

/// Network quality alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAlert {
  /// Alert ID
  pub id: String,
  /// Peer user ID
  pub peer_id: String,
  /// Alert type
  pub alert_type: AlertType,
  /// Alert severity
  pub severity: AlertSeverity,
  /// Alert message
  pub message: String,
  /// Current metrics when alert triggered
  pub metrics: NetworkStats,
  /// Timestamp (ms since epoch)
  pub timestamp: f64,
  /// Whether the alert has been acknowledged
  pub acknowledged: bool,
}

/// Alert type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
  /// High RTT detected
  HighRtt,
  /// High packet loss detected
  HighPacketLoss,
  /// High jitter detected
  HighJitter,
  /// Quality degradation
  QualityDegradation,
  /// Connection unstable
  ConnectionUnstable,
}

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
  /// Warning level
  Warning,
  /// Critical level
  Critical,
}

// =============================================================================
// Peer Network History
// =============================================================================

/// Network history for a single peer
#[derive(Debug, Clone, Default)]
pub struct PeerNetworkHistory {
  /// Historical data points
  pub history: VecDeque<HistoryDataPoint>,
  /// Consecutive poor quality count
  pub consecutive_poor_count: u32,
  /// Last quality level
  pub last_quality: QualityLevel,
}

impl PeerNetworkHistory {
  /// Create new peer history
  #[must_use]
  pub fn new() -> Self {
    Self {
      history: VecDeque::with_capacity(MAX_HISTORY_SIZE),
      consecutive_poor_count: 0,
      last_quality: QualityLevel::default(),
    }
  }

  /// Add a new data point
  pub fn push(&mut self, stats: &NetworkStats) {
    let data_point = HistoryDataPoint::from(stats);

    // Track consecutive poor quality
    if stats.quality == QualityLevel::Poor {
      self.consecutive_poor_count += 1;
    } else {
      self.consecutive_poor_count = 0;
    }

    self.last_quality = stats.quality;

    // Maintain max size
    if self.history.len() >= MAX_HISTORY_SIZE {
      self.history.pop_front();
    }
    self.history.push_back(data_point);
  }

  /// Get average RTT over history
  #[must_use]
  pub fn avg_rtt(&self) -> f64 {
    if self.history.is_empty() {
      return 0.0;
    }
    self.history.iter().map(|h| h.rtt_ms).sum::<f64>() / self.history.len() as f64
  }

  /// Get average packet loss over history
  #[must_use]
  pub fn avg_packet_loss(&self) -> f64 {
    if self.history.is_empty() {
      return 0.0;
    }
    self
      .history
      .iter()
      .map(|h| h.packet_loss_percent)
      .sum::<f64>()
      / self.history.len() as f64
  }

  /// Get average bitrate over history
  #[must_use]
  pub fn avg_bitrate(&self) -> f64 {
    if self.history.is_empty() {
      return 0.0;
    }
    self.history.iter().map(|h| h.bitrate_kbps).sum::<f64>() / self.history.len() as f64
  }

  /// Get min/max RTT
  #[must_use]
  pub fn rtt_range(&self) -> (f64, f64) {
    if self.history.is_empty() {
      return (0.0, 0.0);
    }
    let mut min = f64::MAX;
    let mut max = f64::MIN;
    for h in &self.history {
      min = min.min(h.rtt_ms);
      max = max.max(h.rtt_ms);
    }
    (min, max)
  }

  /// Get quality distribution (excellent, good, fair, poor counts)
  #[must_use]
  pub fn quality_distribution(&self) -> (u32, u32, u32, u32) {
    let mut excellent = 0;
    let mut good = 0;
    let mut fair = 0;
    let mut poor = 0;
    for h in &self.history {
      match h.quality {
        QualityLevel::Excellent => excellent += 1,
        QualityLevel::Good => good += 1,
        QualityLevel::Fair => fair += 1,
        QualityLevel::Poor => poor += 1,
      }
    }
    (excellent, good, fair, poor)
  }
}

// =============================================================================
// Previous Statistics (for delta calculation)
// =============================================================================

/// Previous raw counter values
#[derive(Debug, Clone, Default)]
pub struct PrevCounters {
  pub bytes_sent: f64,
  pub packets_sent: f64,
  pub packets_lost: f64,
  pub timestamp: f64,
}
