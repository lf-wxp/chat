//! Voice Activity Detection (VAD) for the active-speaker indicator.
//!
//! A [`VoiceActivityDetector`] wraps a Web Audio `AnalyserNode`
//! attached to a `MediaStreamAudioSourceNode`. The caller polls
//! [`is_speaking`] on a fixed cadence (usually the same 100–200 ms
//! interval used by the animation loop) to drive the "currently
//! talking" pulse around a participant's tile.
//!
//! The detection algorithm is simple on purpose:
//!
//! 1. Copy the frequency-domain bytes via `getByteFrequencyData`.
//! 2. Compute the mean energy across the first half of bins (voice
//!    energy is concentrated below ~4 kHz; higher bins are mostly
//!    noise / aliasing).
//! 3. Compare against [`ENERGY_THRESHOLD`]; debounce with a short
//!    hang-over window so quick pauses between words do not flicker.
//!
//! No FFT frame accumulation or ML model is used — the goal is a
//! cheap "somebody is talking right now" signal, not accurate speech
//! segmentation.

use wasm_bindgen::JsCast;
use web_sys::{AnalyserNode, AudioContext, MediaStream, MediaStreamAudioSourceNode};

/// Frequency-domain mean-energy threshold above which the peer is
/// considered to be speaking. Empirically tuned against `fftSize = 512`
/// and an `AnalyserNode` with `smoothingTimeConstant = 0.85`.
const ENERGY_THRESHOLD: f64 = 28.0;

/// FFT size requested on the `AnalyserNode`. 512 is a good compromise
/// between frequency resolution and CPU cost.
const FFT_SIZE: u32 = 512;

/// Number of polling windows to hang on to a "speaking" verdict after
/// energy drops below the threshold. Prevents rapid flicker during
/// normal speech pauses.
const HANG_OVER_WINDOWS: u32 = 3;

/// A Web Audio–backed voice-activity detector for a single audio
/// `MediaStream`. Holds the source/analyser nodes alive for as long
/// as the detector is in scope; dropping the detector releases the
/// graph so the browser can reclaim resources.
pub struct VoiceActivityDetector {
  // Context is retained so its lifetime matches the analyser; dropping
  // it would cause the analyser's reads to return silence.
  _ctx: AudioContext,
  _source: MediaStreamAudioSourceNode,
  analyser: AnalyserNode,
  /// Number of bins returned by `getByteFrequencyData` (= `fftSize / 2`).
  bin_count: u32,
  /// Reusable frequency buffer. Pre-allocated in `attach` so polling
  /// at 10 Hz across an 8-peer mesh does not generate ~80 allocations
  /// per second (P2-New-5 fix).
  buffer: Vec<u8>,
  /// Number of remaining hang-over windows.
  hang_over: u32,
}

impl std::fmt::Debug for VoiceActivityDetector {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("VoiceActivityDetector")
      .field("bin_count", &self.bin_count)
      .field("hang_over", &self.hang_over)
      .finish()
  }
}

impl VoiceActivityDetector {
  /// Attach a new detector to `stream`. The stream must contain at
  /// least one audio track.
  ///
  /// # Errors
  /// Returns `Err` when the browser refuses to construct an
  /// `AudioContext` (rare, e.g. the tab was backgrounded before a
  /// user gesture), when the stream has no audio tracks, or when
  /// the analyser cannot be wired up.
  pub fn attach(stream: &MediaStream) -> Result<Self, String> {
    if stream.get_audio_tracks().length() == 0 {
      return Err("Stream has no audio tracks".to_string());
    }

    let ctx = AudioContext::new().map_err(|e| format!("AudioContext failed: {e:?}"))?;
    let source = ctx
      .create_media_stream_source(stream)
      .map_err(|e| format!("createMediaStreamSource failed: {e:?}"))?;
    let analyser = ctx
      .create_analyser()
      .map_err(|e| format!("createAnalyser failed: {e:?}"))?;
    analyser.set_fft_size(FFT_SIZE);
    analyser.set_smoothing_time_constant(0.85);

    source
      .connect_with_audio_node(&analyser)
      .map_err(|e| format!("source.connect(analyser) failed: {e:?}"))?
      .unchecked_into::<AnalyserNode>();

    let bin_count = analyser.frequency_bin_count();

    Ok(Self {
      _ctx: ctx,
      _source: source,
      analyser,
      bin_count,
      buffer: vec![0u8; bin_count as usize],
      hang_over: 0,
    })
  }

  /// Poll the detector once. Returns `true` while the peer is
  /// considered to be speaking.
  ///
  /// Reuses an internal frequency buffer (allocated once in
  /// [`Self::attach`]) so the 10 Hz polling loop produces no
  /// per-tick allocations.
  pub fn is_speaking(&mut self) -> bool {
    // `bin_count` is `fftSize / 2`; we only examine the first half for
    // voice energy (under ~4 kHz at the default sample rate).
    let voice_bins = self.bin_count / 2;
    if self.buffer.len() != self.bin_count as usize {
      self.buffer.resize(self.bin_count as usize, 0);
    }
    self.analyser.get_byte_frequency_data(&mut self.buffer);

    let take = voice_bins as usize;
    if take == 0 {
      return false;
    }
    let total: u64 = self.buffer.iter().take(take).map(|&b| u64::from(b)).sum();
    // Lossless: `take` is bounded by `voice_bins` which comes from a
    // u32 (`fftSize / 2`), well within f64 precision.
    let mean = (total as f64) / (take as f64);

    if mean >= ENERGY_THRESHOLD {
      self.hang_over = HANG_OVER_WINDOWS;
      true
    } else if self.hang_over > 0 {
      self.hang_over -= 1;
      true
    } else {
      false
    }
  }
}
