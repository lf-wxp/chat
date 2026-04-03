//! Network quality monitoring manager
//!
//! Manages periodic collection of network statistics via RTCPeerConnection.getStats().

use std::collections::HashMap;

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::state;

use super::types::{
  AlertSeverity, AlertThresholds, AlertType, NetworkAlert, NetworkStats, PeerNetworkHistory,
  PrevCounters, QualityLevel,
};

// =============================================================================
// Network Quality Manager
// =============================================================================

/// Network quality monitoring manager
///
/// Shared to component tree via `provide_context`.
#[derive(Clone)]
pub struct NetworkQualityManager {
  /// Timer IDs (one per peer)
  timers: StoredValue<HashMap<String, i32>>,
  /// Previous statistics counters
  prev_counters: StoredValue<HashMap<String, PrevCounters>>,
  /// Alert thresholds
  thresholds: AlertThresholds,
}

impl NetworkQualityManager {
  /// Collection interval (ms)
  const POLL_INTERVAL_MS: i32 = 3000;

  /// Create and provide to context
  pub fn provide() {
    let manager = Self {
      timers: StoredValue::new(HashMap::new()),
      prev_counters: StoredValue::new(HashMap::new()),
      thresholds: AlertThresholds::default(),
    };
    provide_context(manager);
  }

  /// Get from context
  pub fn use_manager() -> Self {
    use_context::<Self>().expect("NetworkQualityManager not provided")
  }

  /// Start monitoring specified peer's network quality
  pub fn start_monitoring(&self, remote_user_id: &str) {
    let remote_id = remote_user_id.to_string();

    // Avoid duplicate start
    let already = self.timers.with_value(|t| t.contains_key(&remote_id));
    if already {
      return;
    }

    let self_clone = self.clone();
    let remote_id_timer = remote_id.clone();

    let cb = Closure::<dyn Fn()>::new(move || {
      let remote_id_inner = remote_id_timer.clone();
      let self_inner = self_clone.clone();
      wasm_bindgen_futures::spawn_local(async move {
        self_inner.poll_stats(&remote_id_inner).await;
      });
    });

    let window = web_sys::window().expect("No window");
    let timer_id = window
      .set_interval_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        Self::POLL_INTERVAL_MS,
      )
      .expect("setInterval failed");
    cb.forget();

    self.timers.update_value(|t| {
      t.insert(remote_id.clone(), timer_id);
    });

    web_sys::console::log_1(&format!("[NetworkQuality] Started monitoring: {remote_id}").into());
  }

  /// Stop monitoring specified peer
  pub fn stop_monitoring(&self, remote_user_id: &str) {
    self.timers.update_value(|t| {
      if let Some(timer_id) = t.remove(remote_user_id)
        && let Some(window) = web_sys::window()
      {
        window.clear_interval_with_handle(timer_id);
      }
    });
    self.prev_counters.update_value(|c| {
      c.remove(remote_user_id);
    });

    // Clear state
    let nq_state = state::use_network_quality_state();
    nq_state.update(|s| {
      s.peer_stats.remove(remote_user_id);
      s.peer_history.remove(remote_user_id);
    });
  }

  /// Stop all monitoring
  pub fn stop_all(&self) {
    self.timers.update_value(|t| {
      let window = web_sys::window();
      for timer_id in t.values() {
        if let Some(w) = &window {
          w.clear_interval_with_handle(*timer_id);
        }
      }
      t.clear();
    });
    self.prev_counters.update_value(|c| c.clear());

    let nq_state = state::use_network_quality_state();
    nq_state.update(|s| {
      s.peer_stats.clear();
      s.peer_history.clear();
    });
  }

  /// Collect statistics once
  async fn poll_stats(&self, remote_user_id: &str) {
    let peer_manager = crate::services::webrtc::PeerManager::use_manager();

    // Get RTCPeerConnection
    let pc = peer_manager.get_peer_connection(remote_user_id);
    let Some(pc) = pc else { return };

    // Call getStats()
    let stats_promise = pc.get_stats();
    let Ok(stats_result) = JsFuture::from(stats_promise).await else {
      return;
    };

    let stats_report: web_sys::RtcStatsReport = stats_result.unchecked_into();

    // Parse statistics
    let mut rtt_ms = 0.0_f64;
    let mut available_outgoing_bitrate = 0.0_f64;
    let mut jitter = 0.0_f64;
    let mut total_bytes_sent = 0.0_f64;
    let mut total_packets_sent = 0.0_f64;
    let mut total_packets_lost = 0.0_f64;
    let mut timestamp = 0.0_f64;
    let mut found_candidate_pair = false;
    let mut found_outbound = false;

    // Iterate RTCStatsReport (it's a Map-like object)
    let entries = js_sys::try_iter(&stats_report);
    if let Ok(Some(iter)) = entries {
      for entry in iter {
        let Ok(entry) = entry else { continue };
        let pair: js_sys::Array = entry.unchecked_into();
        let stat_obj = pair.get(1);

        let stat_type = js_sys::Reflect::get(&stat_obj, &"type".into())
          .ok()
          .and_then(|v| v.as_string())
          .unwrap_or_default();

        match stat_type.as_str() {
          "candidate-pair" => {
            // Check if this is the active candidate pair
            let state = js_sys::Reflect::get(&stat_obj, &"state".into())
              .ok()
              .and_then(|v| v.as_string())
              .unwrap_or_default();
            if state != "succeeded" {
              continue;
            }
            found_candidate_pair = true;

            // RTT
            if let Ok(v) = js_sys::Reflect::get(&stat_obj, &"currentRoundTripTime".into())
              && let Some(rtt_sec) = v.as_f64()
            {
              rtt_ms = rtt_sec * 1000.0;
            }

            // Available outgoing bandwidth
            if let Ok(v) = js_sys::Reflect::get(&stat_obj, &"availableOutgoingBitrate".into())
              && let Some(bw) = v.as_f64()
            {
              available_outgoing_bitrate = bw;
            }
          }
          "outbound-rtp" => {
            // Only care about video track
            let kind = js_sys::Reflect::get(&stat_obj, &"kind".into())
              .ok()
              .and_then(|v| v.as_string())
              .unwrap_or_default();
            if kind != "video" {
              continue;
            }
            found_outbound = true;

            if let Ok(v) = js_sys::Reflect::get(&stat_obj, &"bytesSent".into())
              && let Some(bs) = v.as_f64()
            {
              total_bytes_sent = bs;
            }
            if let Ok(v) = js_sys::Reflect::get(&stat_obj, &"packetsSent".into())
              && let Some(ps) = v.as_f64()
            {
              total_packets_sent = ps;
            }
            if let Ok(v) = js_sys::Reflect::get(&stat_obj, &"timestamp".into())
              && let Some(ts) = v.as_f64()
            {
              timestamp = ts;
            }
          }
          "remote-inbound-rtp" => {
            let kind = js_sys::Reflect::get(&stat_obj, &"kind".into())
              .ok()
              .and_then(|v| v.as_string())
              .unwrap_or_default();
            if kind != "video" {
              continue;
            }

            // Packets lost
            if let Ok(v) = js_sys::Reflect::get(&stat_obj, &"packetsLost".into())
              && let Some(pl) = v.as_f64()
            {
              total_packets_lost = pl;
            }

            // Jitter
            if let Ok(v) = js_sys::Reflect::get(&stat_obj, &"jitter".into())
              && let Some(j) = v.as_f64()
            {
              jitter = j * 1000.0; // Convert to ms
            }
          }
          _ => {}
        }
      }
    }

    if !found_candidate_pair && !found_outbound {
      return; // No valid data
    }

    // Calculate packet loss rate and current bitrate
    let remote_id = remote_user_id.to_string();
    let prev = self
      .prev_counters
      .with_value(|c| c.get(&remote_id).cloned());

    let mut packet_loss = 0.0_f64;
    let mut current_bitrate = 0.0_f64;

    if let Some(prev) = &prev {
      let dt_ms = timestamp - prev.timestamp;
      if dt_ms > 0.0 {
        // Packet loss = new losses / new sent
        let delta_sent = total_packets_sent - prev.packets_sent;
        let delta_lost = total_packets_lost - prev.packets_lost;
        if delta_sent > 0.0 {
          packet_loss = (delta_lost / (delta_sent + delta_lost)).clamp(0.0, 1.0);
        }

        // Current bitrate = new bytes * 8 / time delta (seconds)
        let delta_bytes = total_bytes_sent - prev.bytes_sent;
        current_bitrate = (delta_bytes * 8.0) / (dt_ms / 1000.0);
      }
    }

    // Update previous counters
    self.prev_counters.update_value(|c| {
      c.insert(
        remote_id.clone(),
        PrevCounters {
          bytes_sent: total_bytes_sent,
          packets_sent: total_packets_sent,
          packets_lost: total_packets_lost,
          timestamp,
        },
      );
    });

    // Determine quality level
    let quality = QualityLevel::from_stats(rtt_ms, packet_loss);

    let stats = NetworkStats {
      rtt_ms,
      packet_loss,
      available_outgoing_bitrate,
      current_bitrate,
      jitter_ms: jitter,
      quality,
      timestamp,
    };

    // Update global state
    let nq_state = state::use_network_quality_state();
    let prev_quality =
      nq_state.with_untracked(|s| s.peer_stats.get(&remote_id).map(|ps| ps.quality));

    nq_state.update(|s| {
      s.peer_stats.insert(remote_id.clone(), stats.clone());

      // Update history
      let history = s
        .peer_history
        .entry(remote_id.clone())
        .or_insert_with(PeerNetworkHistory::new);
      history.push(&stats);
    });

    // Check for alerts
    self.check_alerts(&remote_id, &stats, prev_quality);

    // If quality level changed, trigger adaptive parameter adjustment
    if prev_quality != Some(quality) {
      web_sys::console::log_1(
        &format!(
          "[NetworkQuality] {}: {} (RTT={:.0}ms, loss={:.1}%, bitrate={:.0}kbps)",
          remote_id,
          quality.label(),
          rtt_ms,
          packet_loss * 100.0,
          current_bitrate / 1000.0,
        )
        .into(),
      );
      Self::apply_adaptive_params(&pc, quality);
    }
  }

  /// Check for network alerts
  fn check_alerts(&self, peer_id: &str, stats: &NetworkStats, prev_quality: Option<QualityLevel>) {
    let nq_state = state::use_network_quality_state();
    let mut alerts = Vec::new();
    let now = js_sys::Date::now();

    // Check for high RTT
    if stats.rtt_ms >= self.thresholds.rtt_critical_ms {
      alerts.push(NetworkAlert {
        id: format!("{}-rtt-critical-{}", peer_id, now as u64),
        peer_id: peer_id.to_string(),
        alert_type: AlertType::HighRtt,
        severity: AlertSeverity::Critical,
        message: format!(
          "Critical RTT: {:.0}ms (threshold: {:.0}ms)",
          stats.rtt_ms, self.thresholds.rtt_critical_ms
        ),
        metrics: stats.clone(),
        timestamp: now,
        acknowledged: false,
      });
    } else if stats.rtt_ms >= self.thresholds.rtt_warning_ms {
      alerts.push(NetworkAlert {
        id: format!("{}-rtt-warning-{}", peer_id, now as u64),
        peer_id: peer_id.to_string(),
        alert_type: AlertType::HighRtt,
        severity: AlertSeverity::Warning,
        message: format!(
          "High RTT: {:.0}ms (threshold: {:.0}ms)",
          stats.rtt_ms, self.thresholds.rtt_warning_ms
        ),
        metrics: stats.clone(),
        timestamp: now,
        acknowledged: false,
      });
    }

    // Check for high packet loss
    if stats.packet_loss >= self.thresholds.packet_loss_critical {
      alerts.push(NetworkAlert {
        id: format!("{}-loss-critical-{}", peer_id, now as u64),
        peer_id: peer_id.to_string(),
        alert_type: AlertType::HighPacketLoss,
        severity: AlertSeverity::Critical,
        message: format!(
          "Critical packet loss: {:.1}% (threshold: {:.1}%)",
          stats.packet_loss * 100.0,
          self.thresholds.packet_loss_critical * 100.0
        ),
        metrics: stats.clone(),
        timestamp: now,
        acknowledged: false,
      });
    } else if stats.packet_loss >= self.thresholds.packet_loss_warning {
      alerts.push(NetworkAlert {
        id: format!("{}-loss-warning-{}", peer_id, now as u64),
        peer_id: peer_id.to_string(),
        alert_type: AlertType::HighPacketLoss,
        severity: AlertSeverity::Warning,
        message: format!(
          "High packet loss: {:.1}% (threshold: {:.1}%)",
          stats.packet_loss * 100.0,
          self.thresholds.packet_loss_warning * 100.0
        ),
        metrics: stats.clone(),
        timestamp: now,
        acknowledged: false,
      });
    }

    // Check for high jitter
    if stats.jitter_ms >= self.thresholds.jitter_critical_ms {
      alerts.push(NetworkAlert {
        id: format!("{}-jitter-critical-{}", peer_id, now as u64),
        peer_id: peer_id.to_string(),
        alert_type: AlertType::HighJitter,
        severity: AlertSeverity::Critical,
        message: format!(
          "Critical jitter: {:.0}ms (threshold: {:.0}ms)",
          stats.jitter_ms, self.thresholds.jitter_critical_ms
        ),
        metrics: stats.clone(),
        timestamp: now,
        acknowledged: false,
      });
    } else if stats.jitter_ms >= self.thresholds.jitter_warning_ms {
      alerts.push(NetworkAlert {
        id: format!("{}-jitter-warning-{}", peer_id, now as u64),
        peer_id: peer_id.to_string(),
        alert_type: AlertType::HighJitter,
        severity: AlertSeverity::Warning,
        message: format!(
          "High jitter: {:.0}ms (threshold: {:.0}ms)",
          stats.jitter_ms, self.thresholds.jitter_warning_ms
        ),
        metrics: stats.clone(),
        timestamp: now,
        acknowledged: false,
      });
    }

    // Check for quality degradation
    if let Some(prev) = prev_quality {
      let degraded = matches!(
        (prev, stats.quality),
        (
          QualityLevel::Excellent,
          QualityLevel::Good | QualityLevel::Fair | QualityLevel::Poor
        ) | (QualityLevel::Good, QualityLevel::Fair | QualityLevel::Poor)
          | (QualityLevel::Fair, QualityLevel::Poor)
      );

      if degraded {
        alerts.push(NetworkAlert {
          id: format!("{}-degraded-{}", peer_id, now as u64),
          peer_id: peer_id.to_string(),
          alert_type: AlertType::QualityDegradation,
          severity: AlertSeverity::Warning,
          message: format!(
            "Quality degraded from {} to {}",
            prev.label(),
            stats.quality.label()
          ),
          metrics: stats.clone(),
          timestamp: now,
          acknowledged: false,
        });
      }
    }

    // Check for consecutive poor quality
    let consecutive_poor = nq_state.with_untracked(|s| {
      s.peer_history
        .get(peer_id)
        .map_or(0, |h| h.consecutive_poor_count)
    });

    if consecutive_poor >= self.thresholds.consecutive_poor_threshold {
      alerts.push(NetworkAlert {
        id: format!("{}-unstable-{}", peer_id, now as u64),
        peer_id: peer_id.to_string(),
        alert_type: AlertType::ConnectionUnstable,
        severity: AlertSeverity::Critical,
        message: format!("Connection unstable: {consecutive_poor} consecutive poor readings"),
        metrics: stats.clone(),
        timestamp: now,
        acknowledged: false,
      });
    }

    // Add alerts to state
    if !alerts.is_empty() {
      nq_state.update(|s| {
        for alert in alerts {
          // Keep only last 50 alerts
          if s.alerts.len() >= 50 {
            s.alerts.remove(0);
          }
          s.alerts.push(alert);
          s.unacknowledged_alert_count += 1;
        }
      });
    }
  }

  /// Adjust video encoding parameters based on quality level
  ///
  /// Since web-sys 0.3.x has incomplete bindings for `RtcRtpSender` `getParameters` / `setParameters`,
  /// we use `js_sys::Reflect` to call JS methods directly.
  fn apply_adaptive_params(pc: &web_sys::RtcPeerConnection, quality: QualityLevel) {
    let senders = pc.get_senders();
    for sender in senders.iter() {
      let Ok(sender) = sender.dyn_into::<web_sys::RtcRtpSender>() else {
        continue;
      };
      let Some(track) = sender.track() else {
        continue;
      };
      if track.kind() != "video" {
        continue;
      }

      // Call sender.getParameters() via Reflect
      let Ok(get_params_fn) = js_sys::Reflect::get(&sender, &"getParameters".into()) else {
        continue;
      };
      let Ok(get_params_fn) = get_params_fn.dyn_into::<js_sys::Function>() else {
        continue;
      };
      let Ok(params) = get_params_fn.call0(&sender) else {
        continue;
      };

      // Modify encodings
      let encodings = js_sys::Reflect::get(&params, &"encodings".into()).ok();
      let Some(encodings) = encodings else { continue };
      let encodings: js_sys::Array = encodings.unchecked_into();

      for i in 0..encodings.length() {
        let encoding = encodings.get(i);

        // Set max bitrate
        let _ = js_sys::Reflect::set(
          &encoding,
          &"maxBitrate".into(),
          &JsValue::from_f64(quality.target_max_bitrate() as f64),
        );

        // Set max framerate
        let _ = js_sys::Reflect::set(
          &encoding,
          &"maxFramerate".into(),
          &JsValue::from_f64(quality.target_max_framerate() as f64),
        );

        // Set scale resolution down
        let _ = js_sys::Reflect::set(
          &encoding,
          &"scaleResolutionDownBy".into(),
          &JsValue::from_f64(quality.target_scale_resolution_down_by()),
        );
      }

      // Call sender.setParameters(params) via Reflect, returns Promise
      let Ok(set_params_fn) = js_sys::Reflect::get(&sender, &"setParameters".into()) else {
        continue;
      };
      let Ok(set_params_fn) = set_params_fn.dyn_into::<js_sys::Function>() else {
        continue;
      };
      let Ok(promise_val) = set_params_fn.call1(&sender, &params) else {
        continue;
      };
      let promise: js_sys::Promise = promise_val.unchecked_into();

      wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = JsFuture::from(promise).await {
          web_sys::console::warn_1(&format!("[NetworkQuality] setParameters failed: {e:?}").into());
        }
      });

      break; // Only process first video sender
    }
  }
}
