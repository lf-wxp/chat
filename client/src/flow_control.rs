//! DataChannel flow control module
//!
//! Back-pressure mechanism based on `bufferedAmount` to prevent buffer overflow
//! caused by sending too fast.
//!
//! ## Core Mechanisms
//!
//! 1. **Water-level detection**: Check the DataChannel's `bufferedAmount` before
//!    sending; pause when it exceeds the high-water mark, resume when it drops
//!    below the low-water mark.
//! 2. **Adaptive rate**: Dynamically adjust the send interval based on buffer
//!
//! ## Testing Note
//!
//! Unit tests are not included in this module because all functions depend on
//! `StoredValue` (Leptos runtime) and `web_sys::RtcDataChannel`, which require
//! a full Leptos reactive context and browser WebRTC environment. These functions
//! are covered by integration / E2E tests instead.
//!    utilization.
//! 3. **Event-driven recovery**: Use the `bufferedamountlow` event to reset the
//!    back-pressure state.
//! 4. **File transfer integration**: Provide flow-control-aware batch sending
//!    for large-file chunked transfers.

use std::collections::HashMap;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

/// High-water mark (256 KB) — pause sending when the buffer exceeds this value
const HIGH_WATER_MARK: u32 = 256 * 1024;

/// Low-water mark (64 KB) — resume sending when the buffer drops below this value
const LOW_WATER_MARK: u32 = 64 * 1024;

/// Maximum retry wait time (milliseconds)
const MAX_WAIT_MS: u32 = 2000;

/// Base wait interval (milliseconds)
const BASE_WAIT_MS: u32 = 10;

/// Maximum chunks per batch (in flow-control mode)
const MAX_CHUNKS_PER_BATCH: usize = 8;

/// Flow-control state for a single channel
#[derive(Default)]
struct ChannelFlowState {
  /// Whether the channel is in back-pressure state (buffer full)
  backpressured: bool,
  /// Consecutive back-pressure count (used for adaptive back-off)
  backpressure_count: u32,
  /// Whether the `bufferedamountlow` event has been registered
  low_event_registered: bool,
}

/// DataChannel flow-control manager
///
/// Manages flow-control state for all DataChannels and provides
/// a back-pressure-aware send API.
#[derive(Clone)]
pub struct FlowController {
  /// Per-peer flow-control state (remote_user_id -> ChannelFlowState)
  states: StoredValue<HashMap<String, ChannelFlowState>>,
}

impl FlowController {
  /// Create and provide to context
  pub fn provide() {
    let controller = Self {
      states: StoredValue::new(HashMap::new()),
    };
    provide_context(controller);
  }

  /// Obtain from context
  #[allow(dead_code)]
  pub fn use_controller() -> Self {
    use_context::<Self>().expect("FlowController not provided")
  }

  /// Check whether the specified channel can send (buffer below high-water mark)
  pub fn can_send(&self, dc: &web_sys::RtcDataChannel) -> bool {
    dc.buffered_amount() < HIGH_WATER_MARK
  }

  /// Get the current buffer utilization ratio (0.0 – 1.0)
  pub fn buffer_usage(&self, dc: &web_sys::RtcDataChannel) -> f64 {
    let buffered = dc.buffered_amount() as f64;
    (buffered / HIGH_WATER_MARK as f64).min(1.0)
  }

  /// Calculate adaptive wait time (based on buffer utilization and consecutive
  /// back-pressure count)
  fn adaptive_wait_ms(&self, peer_id: &str, dc: &web_sys::RtcDataChannel) -> u32 {
    let usage = self.buffer_usage(dc);
    let backpressure_count = self
      .states
      .with_value(|states| states.get(peer_id).map_or(0, |s| s.backpressure_count));

    // Exponential back-off: base * 2^min(count, 6), capped at MAX_WAIT_MS
    let exponent = backpressure_count.min(6);
    let wait = (BASE_WAIT_MS as f64 * (1.0 + usage * 3.0) * (1u32 << exponent) as f64) as u32;
    wait.min(MAX_WAIT_MS)
  }

  /// Register the `bufferedamountlow` event callback (registered only once)
  fn ensure_low_event(&self, peer_id: &str, dc: &web_sys::RtcDataChannel) {
    let already_registered = self
      .states
      .with_value(|states| states.get(peer_id).is_some_and(|s| s.low_event_registered));

    if already_registered {
      return;
    }

    self.states.update_value(|states| {
      let state = states.entry(peer_id.to_string()).or_default();
      state.low_event_registered = true;
    });

    // Set the low-water-mark threshold
    dc.set_buffered_amount_low_threshold(LOW_WATER_MARK);

    let controller = self.clone();
    let peer_id_clone = peer_id.to_string();

    let onbufferedamountlow = Closure::<dyn Fn()>::new(move || {
      web_sys::console::log_1(
        &format!("[FlowCtrl] Buffer below threshold: peer={peer_id_clone}").into(),
      );

      controller.states.update_value(|states| {
        if let Some(state) = states.get_mut(&peer_id_clone) {
          state.backpressured = false;
          state.backpressure_count = 0;
        }
      });
    });

    dc.set_onbufferedamountlow(Some(onbufferedamountlow.as_ref().unchecked_ref()));
    onbufferedamountlow.forget();
  }

  /// Single send with flow control
  ///
  /// If the buffer is not full, send immediately and return `Ok(true)`.
  /// If the buffer is full, return `Ok(false)` indicating a wait is needed.
  pub fn try_send(
    &self,
    peer_id: &str,
    dc: &web_sys::RtcDataChannel,
    data: &[u8],
  ) -> Result<bool, String> {
    // Ensure the low-water-mark event is registered
    self.ensure_low_event(peer_id, dc);

    if dc.ready_state() != web_sys::RtcDataChannelState::Open {
      return Err("DataChannel not open".to_string());
    }

    if self.can_send(dc) {
      // Buffer has space — send immediately
      dc.send_with_u8_array(data)
        .map_err(|e| format!("DataChannel send failed: {e:?}"))?;

      // Send succeeded — decrement back-pressure count
      self.states.update_value(|states| {
        if let Some(state) = states.get_mut(peer_id)
          && state.backpressure_count > 0
        {
          state.backpressure_count = state.backpressure_count.saturating_sub(1);
        }
      });

      Ok(true)
    } else {
      // Buffer full — mark back-pressure
      self.states.update_value(|states| {
        let state = states.entry(peer_id.to_string()).or_default();
        state.backpressured = true;
        state.backpressure_count = state.backpressure_count.saturating_add(1);
      });

      web_sys::console::log_1(
        &format!(
          "[FlowCtrl] Back-pressure triggered: peer={}, buffered={}KB",
          peer_id,
          dc.buffered_amount() / 1024,
        )
        .into(),
      );

      Ok(false)
    }
  }

  /// Async send after waiting for the buffer to become available
  ///
  /// Uses a combination of timed polling and the `bufferedamountlow` event
  /// to resume sending as soon as the buffer is available.
  pub fn send_with_backpressure(&self, peer_id: &str, dc: &web_sys::RtcDataChannel, data: Vec<u8>) {
    match self.try_send(peer_id, dc, &data) {
      Ok(true) => {
        // Sent immediately
      }
      Ok(false) => {
        // Need to wait — start timed retry
        let controller = self.clone();
        let peer_id = peer_id.to_string();
        let dc = dc.clone();
        let wait_ms = self.adaptive_wait_ms(&peer_id, &dc);

        wasm_bindgen_futures::spawn_local(async move {
          controller.wait_and_send(peer_id, dc, data, wait_ms).await;
        });
      }
      Err(e) => {
        web_sys::console::error_1(&format!("[FlowCtrl] Send failed: {e}").into());
      }
    }
  }

  /// Internal: retry sending after a wait
  async fn wait_and_send(
    &self,
    peer_id: String,
    dc: web_sys::RtcDataChannel,
    data: Vec<u8>,
    wait_ms: u32,
  ) {
    gloo_timers::future::sleep(std::time::Duration::from_millis(wait_ms as u64)).await;

    match self.try_send(&peer_id, &dc, &data) {
      Ok(true) => {
        // Send succeeded
      }
      Ok(false) => {
        // Still back-pressured — keep waiting (incremental back-off)
        let next_wait = self.adaptive_wait_ms(&peer_id, &dc);
        let controller = self.clone();
        wasm_bindgen_futures::spawn_local(async move {
          Box::pin(controller.wait_and_send(peer_id, dc, data, next_wait)).await;
        });
      }
      Err(e) => {
        web_sys::console::error_1(&format!("[FlowCtrl] Retry send failed: {e}").into());
      }
    }
  }

  /// Batch send with flow control (for file transfer chunks)
  ///
  /// Sends multiple data chunks in batches, with adaptive waits between
  /// batches based on buffer state.
  pub fn send_chunks_with_flow_control(
    &self,
    peer_id: String,
    dc: web_sys::RtcDataChannel,
    chunks: Vec<Vec<u8>>,
  ) {
    let controller = self.clone();
    wasm_bindgen_futures::spawn_local(async move {
      controller.do_send_chunks(peer_id, dc, chunks, 0).await;
    });
  }

  /// Internal: send chunk data in batches
  async fn do_send_chunks(
    &self,
    peer_id: String,
    dc: web_sys::RtcDataChannel,
    chunks: Vec<Vec<u8>>,
    start_index: usize,
  ) {
    let total = chunks.len();
    let mut index = start_index;

    while index < total {
      // Check DataChannel state
      if dc.ready_state() != web_sys::RtcDataChannelState::Open {
        web_sys::console::error_1(
          &format!(
            "[FlowCtrl] DataChannel closed, stopping send: peer={peer_id}, progress={index}/{total}"
          )
          .into(),
        );
        return;
      }

      // Calculate how many chunks can be sent in this batch
      let batch_size = if self.can_send(&dc) {
        let usage = self.buffer_usage(&dc);
        let dynamic_batch = ((1.0 - usage) * MAX_CHUNKS_PER_BATCH as f64).ceil() as usize;
        dynamic_batch.clamp(1, MAX_CHUNKS_PER_BATCH)
      } else {
        0
      };

      if batch_size == 0 {
        // Buffer full — wait
        let wait_ms = self.adaptive_wait_ms(&peer_id, &dc);
        web_sys::console::log_1(
          &format!("[FlowCtrl] File transfer paused: peer={peer_id}, progress={index}/{total}, waiting {wait_ms}ms")
            .into(),
        );
        gloo_timers::future::sleep(std::time::Duration::from_millis(wait_ms as u64)).await;
        continue;
      }

      // Send this batch
      let batch_end = (index + batch_size).min(total);
      for i in index..batch_end {
        match dc.send_with_u8_array(&chunks[i]) {
          Ok(()) => {}
          Err(e) => {
            web_sys::console::error_1(
              &format!("[FlowCtrl] Chunk send failed: peer={peer_id}, index={i}, err={e:?}").into(),
            );
            // On failure, wait then retry from current position
            gloo_timers::future::sleep(std::time::Duration::from_millis(100)).await;
            let controller = self.clone();
            let peer_id = peer_id.clone();
            let dc = dc.clone();
            wasm_bindgen_futures::spawn_local(async move {
              Box::pin(controller.do_send_chunks(peer_id, dc, chunks, i)).await;
            });
            return;
          }
        }
      }

      index = batch_end;

      // Brief yield between batches to avoid blocking the UI thread
      if index < total {
        let usage = self.buffer_usage(&dc);
        if usage > 0.5 {
          let pause_ms = (usage * 50.0) as u64;
          gloo_timers::future::sleep(std::time::Duration::from_millis(pause_ms)).await;
        } else {
          // Buffer idle — yield once to let the UI refresh
          gloo_timers::future::sleep(std::time::Duration::from_millis(1)).await;
        }
      }
    }

    if index >= total {
      web_sys::console::log_1(
        &format!("[FlowCtrl] Batch send complete: peer={peer_id}, total={total} chunks").into(),
      );
    }
  }

  /// Clean up flow-control state for the specified peer
  pub fn remove_peer(&self, peer_id: &str) {
    self.states.update_value(|states| {
      states.remove(peer_id);
    });
  }

  /// Clean up all flow-control state
  pub fn remove_all(&self) {
    self.states.update_value(|states| {
      states.clear();
    });
  }

  /// Get flow-control statistics for the specified peer
  #[allow(dead_code)]
  pub fn get_stats(&self, peer_id: &str) -> FlowStats {
    self.states.with_value(|states| {
      states
        .get(peer_id)
        .map(|state| FlowStats {
          backpressured: state.backpressured,
          backpressure_count: state.backpressure_count,
        })
        .unwrap_or_default()
    })
  }
}

/// Flow-control statistics
#[derive(Debug, Clone, Default)]
pub struct FlowStats {
  /// Whether the channel is in back-pressure state
  pub backpressured: bool,
  /// Consecutive back-pressure count
  pub backpressure_count: u32,
}
