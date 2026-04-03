//! Network quality monitoring state

use std::collections::HashMap;

/// Network quality state
#[derive(Debug, Clone, Default)]
pub struct NetworkQualityState {
  /// Network statistics per peer
  pub peer_stats: HashMap<String, crate::network_quality::NetworkStats>,
  /// Historical data per peer
  pub peer_history: HashMap<String, crate::network_quality::PeerNetworkHistory>,
  /// Active alerts
  pub alerts: Vec<crate::network_quality::NetworkAlert>,
  /// Unacknowledged alert count
  pub unacknowledged_alert_count: u32,
}

impl NetworkQualityState {
  /// Get quality level for specified peer
  #[must_use]
  pub fn quality(&self, user_id: &str) -> crate::network_quality::QualityLevel {
    self
      .peer_stats
      .get(user_id)
      .map(|s| s.quality)
      .unwrap_or_default()
  }

  /// Get worst quality level among all peers
  #[must_use]
  pub fn worst_quality(&self) -> crate::network_quality::QualityLevel {
    self
      .peer_stats
      .values()
      .map(|s| s.quality)
      .min_by_key(|q| match q {
        crate::network_quality::QualityLevel::Excellent => 3,
        crate::network_quality::QualityLevel::Good => 2,
        crate::network_quality::QualityLevel::Fair => 1,
        crate::network_quality::QualityLevel::Poor => 0,
      })
      .unwrap_or_default()
  }

  /// Acknowledge an alert
  pub fn acknowledge_alert(&mut self, alert_id: &str) {
    if let Some(alert) = self.alerts.iter_mut().find(|a| a.id == alert_id)
      && !alert.acknowledged
    {
      alert.acknowledged = true;
      self.unacknowledged_alert_count = self.unacknowledged_alert_count.saturating_sub(1);
    }
  }

  /// Acknowledge all alerts
  pub fn acknowledge_all_alerts(&mut self) {
    for alert in &mut self.alerts {
      alert.acknowledged = true;
    }
    self.unacknowledged_alert_count = 0;
  }

  /// Clear old acknowledged alerts
  pub fn clear_acknowledged_alerts(&mut self) {
    self.alerts.retain(|a| !a.acknowledged);
  }

  /// Get critical alerts
  #[must_use]
  pub fn critical_alerts(&self) -> Vec<&crate::network_quality::NetworkAlert> {
    self
      .alerts
      .iter()
      .filter(|a| a.severity == crate::network_quality::AlertSeverity::Critical)
      .collect()
  }

  /// Get warning alerts
  #[must_use]
  pub fn warning_alerts(&self) -> Vec<&crate::network_quality::NetworkAlert> {
    self
      .alerts
      .iter()
      .filter(|a| a.severity == crate::network_quality::AlertSeverity::Warning)
      .collect()
  }
}
