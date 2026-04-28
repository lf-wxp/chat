//! Pure-logic helpers for the voice recorder (Task 16).
//!
//! Everything in this module is intentionally native-compatible so it
//! can be exercised by `cargo test` without a browser event loop. The
//! WASM-only orchestration (`MediaRecorder`, `AudioContext`,
//! `requestAnimationFrame`) lives in the parent `voice_recorder`
//! module.

/// Maximum voice clip duration in milliseconds (Req 2 — Task 16).
///
/// Kept in sync with [`crate::chat::models::MAX_VOICE_DURATION_MS`] but
/// re-exported here so the recorder UI does not depend on the chat
/// model crate for a single constant.
pub const MAX_DURATION_MS: u32 = 120_000;

/// Target number of samples retained in the final waveform payload
/// delivered to `ChatManager::send_voice`.
///
/// 60 samples keeps the payload under 60 bytes (one u8 per bar) while
/// still rendering a visually-rich bar chart in the message bubble.
pub const FINAL_SAMPLE_COUNT: usize = 60;

/// Interval between live waveform samples, in milliseconds.
///
/// 33 ms ≈ 30 Hz which matches the 30 fps visualisation requirement in
/// the task checklist while keeping CPU cost negligible.
pub const SAMPLE_INTERVAL_MS: i64 = 33;

/// Collect normalised loudness samples (0..=255) over time for the
/// live waveform visualisation and the final payload.
#[derive(Debug, Default, Clone)]
pub struct WaveformAggregator {
  samples: Vec<u8>,
  last_sample_ms: Option<i64>,
}

impl WaveformAggregator {
  /// Create an empty aggregator.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      samples: Vec::new(),
      last_sample_ms: None,
    }
  }

  /// Return `true` when `now_ms` is at least
  /// [`SAMPLE_INTERVAL_MS`] past the previously captured sample.
  #[must_use]
  pub fn should_sample(&self, now_ms: i64) -> bool {
    match self.last_sample_ms {
      None => true,
      Some(last) => now_ms.saturating_sub(last) >= SAMPLE_INTERVAL_MS,
    }
  }

  /// Append a normalised loudness sample (0..=255) and advance the
  /// sampling clock. Callers must have verified [`should_sample`]
  /// themselves — this method does not rate-limit.
  pub fn push(&mut self, rms: u8, now_ms: i64) {
    self.samples.push(rms);
    self.last_sample_ms = Some(now_ms);
  }

  /// Number of samples currently stored.
  #[must_use]
  pub fn len(&self) -> usize {
    self.samples.len()
  }

  /// Whether the aggregator has collected any samples yet.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.samples.is_empty()
  }

  /// Borrow the raw sample buffer for live rendering.
  #[must_use]
  pub fn samples(&self) -> &[u8] {
    &self.samples
  }

  /// Return the most recent `n` samples in chronological order.
  ///
  /// When fewer than `n` samples exist the full buffer is returned.
  /// Callers use this to drive the bar-chart renderer which only
  /// has room for a fixed window of recent bars.
  #[must_use]
  pub fn tail(&self, n: usize) -> &[u8] {
    let len = self.samples.len();
    if len <= n {
      &self.samples
    } else {
      &self.samples[len - n..]
    }
  }

  /// Produce a fixed-length waveform payload by downsampling the
  /// accumulated buffer to exactly [`FINAL_SAMPLE_COUNT`] buckets
  /// using a mean filter.
  ///
  /// The result is always `FINAL_SAMPLE_COUNT` bytes long so the UI
  /// receives a stable shape regardless of clip duration. Empty
  /// buffers produce a flat zero-filled waveform.
  #[must_use]
  pub fn downsample_final(&self) -> Vec<u8> {
    downsample_mean(&self.samples, FINAL_SAMPLE_COUNT)
  }
}

/// Downsample `src` to exactly `target` samples using a bucketed mean.
///
/// Exposed separately so tests can exercise the algorithm without
/// constructing a [`WaveformAggregator`].
#[must_use]
pub fn downsample_mean(src: &[u8], target: usize) -> Vec<u8> {
  if target == 0 {
    return Vec::new();
  }
  if src.is_empty() {
    return vec![0u8; target];
  }
  if src.len() <= target {
    // Pad with zeros on the right so every slot is populated.
    let mut out = Vec::with_capacity(target);
    out.extend_from_slice(src);
    out.resize(target, 0u8);
    return out;
  }

  let mut out = Vec::with_capacity(target);
  let len = src.len();
  for i in 0..target {
    let start = i * len / target;
    let end = ((i + 1) * len / target).max(start + 1).min(len);
    let slice = &src[start..end];
    let sum: u32 = slice.iter().map(|&b| u32::from(b)).sum();
    let mean = (sum / slice.len() as u32) as u8;
    out.push(mean);
  }
  out
}

/// Compute a normalised RMS-style loudness byte from an
/// `AnalyserNode::getByteFrequencyData` buffer.
///
/// The browser returns unsigned bytes in `[0, 255]`; we average across
/// the spectrum and clamp. Callers push the result into
/// [`WaveformAggregator::push`].
#[must_use]
pub fn average_loudness(fft_bytes: &[u8]) -> u8 {
  if fft_bytes.is_empty() {
    return 0;
  }
  let sum: u32 = fft_bytes.iter().map(|&b| u32::from(b)).sum();
  let avg = sum / fft_bytes.len() as u32;
  avg.min(255) as u8
}

/// Recording state surfaced to the UI layer.
///
/// The transition graph is:
/// `Idle → Starting → Recording → Stopping → Idle`.
/// Every transition that moves away from `Recording` is idempotent so
/// a stray callback (e.g. `MediaRecorder::onstop` firing twice) is
/// safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingState {
  /// Ready to start — the overlay is open but no capture is running.
  Idle,
  /// `getUserMedia` or `MediaRecorder.start` is in flight.
  Starting,
  /// Capture is live; RAF loop is driving the waveform.
  Recording,
  /// `MediaRecorder.stop` has been requested; blobs are being flushed.
  Stopping,
}

impl RecordingState {
  /// Whether a transition to `Recording` should be accepted.
  #[must_use]
  pub const fn can_start(self) -> bool {
    matches!(self, Self::Idle)
  }

  /// Whether a transition to `Stopping` should be accepted.
  #[must_use]
  pub const fn can_stop(self) -> bool {
    matches!(self, Self::Recording)
  }
}

#[cfg(test)]
mod tests;
