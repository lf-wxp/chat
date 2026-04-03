use super::*;
use crate::network_quality::types::{AlertThresholds, HistoryDataPoint, MAX_HISTORY_SIZE};
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

// =========================================================================
// QualityLevel::from_stats tests
// =========================================================================

#[wasm_bindgen_test]
fn test_quality_excellent() {
  assert_eq!(
    QualityLevel::from_stats(10.0, 0.005),
    QualityLevel::Excellent
  );
  assert_eq!(
    QualityLevel::from_stats(49.9, 0.009),
    QualityLevel::Excellent
  );
}

#[wasm_bindgen_test]
fn test_quality_good() {
  assert_eq!(QualityLevel::from_stats(50.0, 0.005), QualityLevel::Good);
  assert_eq!(QualityLevel::from_stats(100.0, 0.02), QualityLevel::Good);
  assert_eq!(QualityLevel::from_stats(149.0, 0.029), QualityLevel::Good);
}

#[wasm_bindgen_test]
fn test_quality_fair() {
  assert_eq!(QualityLevel::from_stats(150.0, 0.03), QualityLevel::Fair);
  assert_eq!(QualityLevel::from_stats(250.0, 0.05), QualityLevel::Fair);
  assert_eq!(QualityLevel::from_stats(299.0, 0.079), QualityLevel::Fair);
}

#[wasm_bindgen_test]
fn test_quality_poor() {
  assert_eq!(QualityLevel::from_stats(300.0, 0.01), QualityLevel::Poor);
  assert_eq!(QualityLevel::from_stats(100.0, 0.08), QualityLevel::Poor);
  assert_eq!(QualityLevel::from_stats(500.0, 0.15), QualityLevel::Poor);
}

#[wasm_bindgen_test]
fn test_quality_boundary_rtt_50_loss_0() {
  // RTT = 50ms, loss = 0% → Good (doesn't meet Excellent RTT < 50)
  assert_eq!(QualityLevel::from_stats(50.0, 0.0), QualityLevel::Good);
}

#[wasm_bindgen_test]
fn test_quality_boundary_rtt_0_loss_1pct() {
  // RTT = 0ms, loss = 1% → Good (doesn't meet Excellent loss < 1%)
  assert_eq!(QualityLevel::from_stats(0.0, 0.01), QualityLevel::Good);
}

#[wasm_bindgen_test]
fn test_quality_zero_values() {
  assert_eq!(QualityLevel::from_stats(0.0, 0.0), QualityLevel::Excellent);
}

// =========================================================================
// QualityLevel property method tests
// =========================================================================

#[wasm_bindgen_test]
fn test_quality_label() {
  assert_eq!(QualityLevel::Excellent.label(), "Excellent");
  assert_eq!(QualityLevel::Good.label(), "Good");
  assert_eq!(QualityLevel::Fair.label(), "Fair");
  assert_eq!(QualityLevel::Poor.label(), "Poor");
}

#[wasm_bindgen_test]
fn test_quality_css_class() {
  assert_eq!(QualityLevel::Excellent.css_class(), "excellent");
  assert_eq!(QualityLevel::Good.css_class(), "good");
  assert_eq!(QualityLevel::Fair.css_class(), "fair");
  assert_eq!(QualityLevel::Poor.css_class(), "poor");
}

#[wasm_bindgen_test]
fn test_quality_color() {
  assert_eq!(QualityLevel::Excellent.color(), "#22c55e");
  assert_eq!(QualityLevel::Good.color(), "#84cc16");
  assert_eq!(QualityLevel::Fair.color(), "#eab308");
  assert_eq!(QualityLevel::Poor.color(), "#ef4444");
}

#[wasm_bindgen_test]
fn test_quality_icon() {
  // All levels use same icon
  assert_eq!(QualityLevel::Excellent.icon(), "📶");
  assert_eq!(QualityLevel::Poor.icon(), "📶");
}

#[wasm_bindgen_test]
fn test_quality_default() {
  assert_eq!(QualityLevel::default(), QualityLevel::Good);
}

// =========================================================================
// Adaptive parameter tests
// =========================================================================

#[wasm_bindgen_test]
fn test_target_max_bitrate() {
  assert_eq!(QualityLevel::Excellent.target_max_bitrate(), 2_500_000);
  assert_eq!(QualityLevel::Good.target_max_bitrate(), 1_500_000);
  assert_eq!(QualityLevel::Fair.target_max_bitrate(), 800_000);
  assert_eq!(QualityLevel::Poor.target_max_bitrate(), 300_000);
}

#[wasm_bindgen_test]
fn test_target_max_framerate() {
  assert_eq!(QualityLevel::Excellent.target_max_framerate(), 30);
  assert_eq!(QualityLevel::Good.target_max_framerate(), 24);
  assert_eq!(QualityLevel::Fair.target_max_framerate(), 15);
  assert_eq!(QualityLevel::Poor.target_max_framerate(), 10);
}

#[wasm_bindgen_test]
fn test_target_scale_resolution() {
  assert!((QualityLevel::Excellent.target_scale_resolution_down_by() - 1.0).abs() < f64::EPSILON);
  assert!((QualityLevel::Good.target_scale_resolution_down_by() - 1.0).abs() < f64::EPSILON);
  assert!((QualityLevel::Fair.target_scale_resolution_down_by() - 1.5).abs() < f64::EPSILON);
  assert!((QualityLevel::Poor.target_scale_resolution_down_by() - 2.0).abs() < f64::EPSILON);
}

// =========================================================================
// NetworkStats default tests
// =========================================================================

#[wasm_bindgen_test]
fn test_network_stats_default() {
  let stats = NetworkStats::default();
  assert!((stats.rtt_ms - 0.0).abs() < f64::EPSILON);
  assert!((stats.packet_loss - 0.0).abs() < f64::EPSILON);
  assert_eq!(stats.quality, QualityLevel::default());
}

// =========================================================================
// PeerNetworkHistory tests
// =========================================================================

#[wasm_bindgen_test]
fn test_peer_history_new() {
  let history = PeerNetworkHistory::new();
  assert!(history.history.is_empty());
  assert_eq!(history.consecutive_poor_count, 0);
  assert_eq!(history.last_quality, QualityLevel::Good);
}

#[wasm_bindgen_test]
fn test_peer_history_push() {
  let mut history = PeerNetworkHistory::new();
  let stats = NetworkStats {
    rtt_ms: 100.0,
    packet_loss: 0.02,
    current_bitrate: 1_000_000.0,
    jitter_ms: 10.0,
    quality: QualityLevel::Good,
    timestamp: 1000.0,
    ..Default::default()
  };

  history.push(&stats);
  assert_eq!(history.history.len(), 1);
  assert_eq!(history.last_quality, QualityLevel::Good);
  assert_eq!(history.consecutive_poor_count, 0);
}

#[wasm_bindgen_test]
fn test_peer_history_consecutive_poor() {
  let mut history = PeerNetworkHistory::new();
  let poor_stats = NetworkStats {
    quality: QualityLevel::Poor,
    timestamp: 1000.0,
    ..Default::default()
  };

  history.push(&poor_stats);
  assert_eq!(history.consecutive_poor_count, 1);

  history.push(&poor_stats);
  assert_eq!(history.consecutive_poor_count, 2);

  let good_stats = NetworkStats {
    quality: QualityLevel::Good,
    timestamp: 2000.0,
    ..Default::default()
  };
  history.push(&good_stats);
  assert_eq!(history.consecutive_poor_count, 0);
}

#[wasm_bindgen_test]
fn test_peer_history_max_size() {
  let mut history = PeerNetworkHistory::new();
  let stats = NetworkStats {
    timestamp: 1000.0,
    ..Default::default()
  };

  for i in 0..(MAX_HISTORY_SIZE + 10) {
    let mut s = stats.clone();
    s.timestamp = 1000.0 + i as f64 * 100.0;
    history.push(&s);
  }

  assert_eq!(history.history.len(), MAX_HISTORY_SIZE);
}

#[wasm_bindgen_test]
fn test_peer_history_avg_rtt() {
  let mut history = PeerNetworkHistory::new();
  for i in 0..5 {
    history.push(&NetworkStats {
      rtt_ms: 100.0 + i as f64 * 10.0,
      timestamp: i as f64 * 1000.0,
      ..Default::default()
    });
  }

  let avg = history.avg_rtt();
  assert!((avg - 120.0).abs() < f64::EPSILON);
}

#[wasm_bindgen_test]
fn test_peer_history_quality_distribution() {
  let mut history = PeerNetworkHistory::new();
  history.push(&NetworkStats {
    quality: QualityLevel::Excellent,
    timestamp: 1.0,
    ..Default::default()
  });
  history.push(&NetworkStats {
    quality: QualityLevel::Excellent,
    timestamp: 2.0,
    ..Default::default()
  });
  history.push(&NetworkStats {
    quality: QualityLevel::Good,
    timestamp: 3.0,
    ..Default::default()
  });
  history.push(&NetworkStats {
    quality: QualityLevel::Fair,
    timestamp: 4.0,
    ..Default::default()
  });
  history.push(&NetworkStats {
    quality: QualityLevel::Poor,
    timestamp: 5.0,
    ..Default::default()
  });

  let (excellent, good, fair, poor) = history.quality_distribution();
  assert_eq!(excellent, 2);
  assert_eq!(good, 1);
  assert_eq!(fair, 1);
  assert_eq!(poor, 1);
}

// =========================================================================
// HistoryDataPoint tests
// =========================================================================

#[wasm_bindgen_test]
fn test_peer_history_avg_packet_loss() {
  let mut history = PeerNetworkHistory::new();
  for i in 0..5 {
    history.push(&NetworkStats {
      packet_loss: 0.01 * (i + 1) as f64, // 1%, 2%, 3%, 4%, 5%
      timestamp: i as f64 * 1000.0,
      ..Default::default()
    });
  }

  let avg = history.avg_packet_loss();
  // Average of 1%, 2%, 3%, 4%, 5% = 3.0 (stored as percentage)
  assert!((avg - 3.0).abs() < f64::EPSILON);
}

#[wasm_bindgen_test]
fn test_peer_history_avg_packet_loss_empty() {
  let history = PeerNetworkHistory::new();
  assert!((history.avg_packet_loss() - 0.0).abs() < f64::EPSILON);
}

#[wasm_bindgen_test]
fn test_peer_history_avg_bitrate() {
  let mut history = PeerNetworkHistory::new();
  for i in 0..4 {
    history.push(&NetworkStats {
      current_bitrate: 1_000_000.0 * (i + 1) as f64, // 1M, 2M, 3M, 4M bps
      timestamp: i as f64 * 1000.0,
      ..Default::default()
    });
  }

  let avg = history.avg_bitrate();
  // Average of 1000, 2000, 3000, 4000 kbps = 2500 kbps
  assert!((avg - 2500.0).abs() < f64::EPSILON);
}

#[wasm_bindgen_test]
fn test_peer_history_avg_bitrate_empty() {
  let history = PeerNetworkHistory::new();
  assert!((history.avg_bitrate() - 0.0).abs() < f64::EPSILON);
}

#[wasm_bindgen_test]
fn test_peer_history_rtt_range() {
  let mut history = PeerNetworkHistory::new();
  let rtt_values = [50.0, 120.0, 30.0, 200.0, 80.0];
  for (i, &rtt) in rtt_values.iter().enumerate() {
    history.push(&NetworkStats {
      rtt_ms: rtt,
      timestamp: i as f64 * 1000.0,
      ..Default::default()
    });
  }

  let (min, max) = history.rtt_range();
  assert!((min - 30.0).abs() < f64::EPSILON);
  assert!((max - 200.0).abs() < f64::EPSILON);
}

#[wasm_bindgen_test]
fn test_peer_history_rtt_range_empty() {
  let history = PeerNetworkHistory::new();
  let (min, max) = history.rtt_range();
  assert!((min - 0.0).abs() < f64::EPSILON);
  assert!((max - 0.0).abs() < f64::EPSILON);
}

#[wasm_bindgen_test]
fn test_peer_history_rtt_range_single() {
  let mut history = PeerNetworkHistory::new();
  history.push(&NetworkStats {
    rtt_ms: 100.0,
    timestamp: 1000.0,
    ..Default::default()
  });

  let (min, max) = history.rtt_range();
  assert!((min - 100.0).abs() < f64::EPSILON);
  assert!((max - 100.0).abs() < f64::EPSILON);
}

// =========================================================================
// HistoryDataPoint tests
// =========================================================================

#[wasm_bindgen_test]
fn test_history_data_point_from_stats() {
  let stats = NetworkStats {
    rtt_ms: 100.0,
    packet_loss: 0.05,
    current_bitrate: 1_500_000.0,
    jitter_ms: 20.0,
    quality: QualityLevel::Good,
    timestamp: 12345.0,
    ..Default::default()
  };

  let point = HistoryDataPoint::from(&stats);
  assert!((point.rtt_ms - 100.0).abs() < f64::EPSILON);
  assert!((point.packet_loss_percent - 5.0).abs() < f64::EPSILON);
  assert!((point.bitrate_kbps - 1500.0).abs() < f64::EPSILON);
  assert!((point.jitter_ms - 20.0).abs() < f64::EPSILON);
  assert_eq!(point.quality, QualityLevel::Good);
}

// =========================================================================
// AlertThresholds tests
// =========================================================================

#[wasm_bindgen_test]
fn test_alert_thresholds_default() {
  let thresholds = AlertThresholds::default();
  assert!((thresholds.rtt_warning_ms - 150.0).abs() < f64::EPSILON);
  assert!((thresholds.rtt_critical_ms - 300.0).abs() < f64::EPSILON);
  assert!((thresholds.packet_loss_warning - 0.03).abs() < f64::EPSILON);
  assert!((thresholds.packet_loss_critical - 0.08).abs() < f64::EPSILON);
  assert_eq!(thresholds.consecutive_poor_threshold, 3);
}
