//! Exponential backoff reconnection strategy.
//!
//! Implements the reconnection logic described in Req 1.8:
//! "WHEN a WebSocket connection drops unexpectedly THEN the client
//! SHALL automatically attempt reconnection (exponential backoff strategy)"

use std::time::Duration;

/// Maximum number of reconnection attempts before giving up.
const MAX_RECONNECT_ATTEMPTS: u32 = 10;

/// Base delay for exponential backoff (1 second).
const BASE_DELAY: Duration = Duration::from_secs(1);

/// Maximum delay between reconnection attempts (30 seconds).
const MAX_DELAY: Duration = Duration::from_secs(30);

/// Source of `[0.0, 1.0)` random samples used for jitter.
///
/// Extracted as a trait so tests can inject deterministic sequences and
/// exercise the full ±10% jitter range instead of clamping to the midpoint
/// (P2-1 fix).
pub trait RandSource {
  /// Return a pseudo-random `f64` in `[0.0, 1.0)`.
  fn next_f64(&mut self) -> f64;
}

/// Default random source: uses `js_sys::Math::random()` on WASM targets
/// and a deterministic 0.5 midpoint on native targets (tests should
/// inject their own source via [`ReconnectStrategy::with_rand`]).
#[derive(Debug, Clone, Default)]
pub struct DefaultRand;

impl RandSource for DefaultRand {
  fn next_f64(&mut self) -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
      js_sys::Math::random()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      0.5
    }
  }
}

/// Minimal seeded PRNG (xorshift64*) for deterministic tests.
///
/// Not cryptographically secure; the jitter it feeds never affects
/// production security and 64-bit xorshift is ample for sampling
/// reconnect delays.
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct SeededRand {
  state: u64,
}

#[cfg(test)]
impl SeededRand {
  /// Construct a new seeded generator. Seed 0 is remapped to a
  /// non-zero constant because xorshift cannot escape 0.
  #[must_use]
  pub fn new(seed: u64) -> Self {
    Self {
      state: if seed == 0 {
        0x9E37_79B9_7F4A_7C15
      } else {
        seed
      },
    }
  }
}

#[cfg(test)]
impl RandSource for SeededRand {
  fn next_f64(&mut self) -> f64 {
    // xorshift64*
    let mut x = self.state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    self.state = x;
    let mixed = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
    // Use top 53 bits for uniform f64 in [0, 1).
    ((mixed >> 11) as f64) / ((1u64 << 53) as f64)
  }
}

/// Exponential backoff reconnection strategy.
///
/// Delays follow the pattern: 1s, 2s, 4s, 8s, 16s, 30s, 30s, ...
/// After `MAX_RECONNECT_ATTEMPTS` failed attempts, stops trying.
///
/// The `R` type parameter threads a [`RandSource`] through the strategy
/// so tests can inject a seeded generator. Production code uses
/// [`DefaultRand`] via [`ReconnectStrategy::new`] and never needs to
/// think about it.
#[derive(Debug, Clone)]
pub struct ReconnectStrategy<R: RandSource = DefaultRand> {
  /// Current attempt number (0 = first attempt).
  attempt: u32,
  /// Whether reconnection is stopped (user logout or max attempts).
  stopped: bool,
  /// Source of jitter samples.
  rand: R,
}

impl ReconnectStrategy<DefaultRand> {
  /// Create a new reconnect strategy starting at attempt 0.
  pub fn new() -> Self {
    Self::with_rand(DefaultRand)
  }
}

impl<R: RandSource> ReconnectStrategy<R> {
  /// Create a strategy with a caller-supplied [`RandSource`].
  ///
  /// Useful for deterministic tests. Production code should prefer
  /// [`ReconnectStrategy::new`].
  pub fn with_rand(rand: R) -> Self {
    Self {
      attempt: 0,
      stopped: false,
      rand,
    }
  }

  /// Get the next reconnection delay.
  ///
  /// Returns `None` if the maximum number of attempts has been reached
  /// or if reconnection has been stopped.
  pub fn next_delay(&mut self) -> Option<Duration> {
    if self.stopped || self.attempt >= MAX_RECONNECT_ATTEMPTS {
      return None;
    }

    // Exponential backoff with jitter: base * 2^attempt
    let exponential = BASE_DELAY.as_millis() as u64 * 2u64.pow(self.attempt);
    let capped = exponential.min(MAX_DELAY.as_millis() as u64);

    // Add random jitter (±10%) to prevent thundering herd.
    // The jitter shifts the delay by -10%..+10% of the capped value.
    let jitter_ms = if capped > 100 {
      let jitter_range = (capped / 10) as f64;
      // Use `next_f64()` so tests can inject a seeded generator
      // (P2-1 fix). The default source calls `Math.random()` on WASM.
      let random_factor = self.rand.next_f64();
      // Map [0,1) → [-1, +1) then scale to jitter_range
      (random_factor * 2.0 - 1.0) * jitter_range
    } else {
      0.0
    };

    let delay_ms = (capped as f64 + jitter_ms).max(0.0) as u64;
    self.attempt += 1;

    Some(Duration::from_millis(delay_ms))
  }

  /// Reset the strategy after a successful connection.
  pub fn reset(&mut self) {
    self.attempt = 0;
    self.stopped = false;
  }

  /// Stop reconnection attempts (e.g., on user logout).
  pub fn stop(&mut self) {
    self.stopped = true;
  }

  /// Get the current attempt number (0-based internal counter).
  ///
  /// Primarily used in tests; production code should prefer
  /// [`display_attempt`] for human-readable output.
  #[must_use]
  #[allow(dead_code)]
  pub fn attempt(&self) -> u32 {
    self.attempt
  }

  /// Get the 1-based attempt number for display purposes.
  ///
  /// Returns `attempt + 1` so callers don't need the `attempt + 1` hack
  /// in log messages (Issue-10 fix).
  #[must_use]
  pub fn display_attempt(&self) -> u32 {
    self.attempt + 1
  }

  /// Check if reconnection is stopped.
  #[must_use]
  pub fn is_stopped(&self) -> bool {
    self.stopped
  }
}

impl Default for ReconnectStrategy<DefaultRand> {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests;
