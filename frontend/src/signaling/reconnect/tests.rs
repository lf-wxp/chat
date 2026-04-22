use super::*;

/// Deterministic "constant 0.5" source so legacy tests keep passing.
#[derive(Clone)]
struct ConstRand(f64);
impl RandSource for ConstRand {
  fn next_f64(&mut self) -> f64 {
    self.0
  }
}

#[test]
fn test_first_delay_is_near_base() {
  let mut strategy = ReconnectStrategy::with_rand(ConstRand(0.5));
  let delay = strategy.next_delay().unwrap();
  // With random_factor=0.5, jitter = (0.5*2-1)*range = 0, so delay == base
  assert_eq!(delay, BASE_DELAY);
}

#[test]
fn test_delays_increase_exponentially() {
  let mut strategy = ReconnectStrategy::with_rand(ConstRand(0.5));
  let mut delays = Vec::new();
  for _ in 0..5 {
    if let Some(d) = strategy.next_delay() {
      delays.push(d);
    }
  }
  // Each delay should be >= the previous (with 0.5 factor, jitter is 0)
  for i in 1..delays.len() {
    assert!(delays[i] >= delays[i - 1]);
  }
}

#[test]
fn test_max_delay_is_capped() {
  let mut strategy = ReconnectStrategy::with_rand(ConstRand(0.99));
  for _ in 0..10 {
    if let Some(delay) = strategy.next_delay() {
      // With ±10% jitter, max possible is MAX_DELAY + 10%
      assert!(delay <= MAX_DELAY + MAX_DELAY / 10);
    }
  }
}

#[test]
fn test_max_attempts_then_none() {
  let mut strategy = ReconnectStrategy::with_rand(ConstRand(0.5));
  for _ in 0..MAX_RECONNECT_ATTEMPTS {
    assert!(strategy.next_delay().is_some());
  }
  assert!(strategy.next_delay().is_none());
}

#[test]
fn test_stop_prevents_reconnect() {
  let mut strategy = ReconnectStrategy::with_rand(ConstRand(0.5));
  strategy.stop();
  assert!(strategy.next_delay().is_none());
}

#[test]
fn test_reset_allows_reconnect() {
  let mut strategy = ReconnectStrategy::with_rand(ConstRand(0.5));
  for _ in 0..MAX_RECONNECT_ATTEMPTS {
    strategy.next_delay();
  }
  assert!(strategy.next_delay().is_none());
  strategy.reset();
  assert!(strategy.next_delay().is_some());
  assert_eq!(strategy.attempt(), 1);
}

#[test]
fn test_display_attempt_is_one_based() {
  let mut strategy = ReconnectStrategy::with_rand(ConstRand(0.5));
  assert_eq!(strategy.display_attempt(), 1);
  strategy.next_delay();
  assert_eq!(strategy.display_attempt(), 2);
}

// ── P2-1 new tests: seeded RNG covers the ±10% jitter range ──

#[test]
fn test_jitter_lower_bound_minus_10_percent() {
  // factor=0.0 → jitter = (0*2-1)*range = -range → delay = base - 10%
  let mut s = ReconnectStrategy::with_rand(ConstRand(0.0));
  let delay = s.next_delay().unwrap();
  let expected = BASE_DELAY.as_millis() as u64 * 9 / 10;
  assert_eq!(
    delay.as_millis() as u64,
    expected,
    "Min jitter should subtract 10%"
  );
}

#[test]
fn test_jitter_upper_bound_plus_10_percent() {
  // factor just under 1.0 → jitter ≈ +range (<100ms below cap).
  let mut s = ReconnectStrategy::with_rand(ConstRand(0.999_999));
  let delay = s.next_delay().unwrap();
  let base_ms = BASE_DELAY.as_millis() as u64;
  let max_expected = base_ms + base_ms / 10; // +10%
  assert!(
    delay.as_millis() as u64 <= max_expected,
    "Max jitter must not exceed +10% (got {}ms, max {}ms)",
    delay.as_millis(),
    max_expected
  );
  assert!(
    delay.as_millis() as u64 > base_ms,
    "Factor ~1.0 should push delay above base"
  );
}

#[test]
fn test_seeded_rand_produces_varied_samples() {
  // Drive the strategy with a seeded xorshift source and assert that
  // we see at least two distinct delays for attempts with the same
  // exponential tier (both capped at MAX_DELAY). This protects us
  // against a regression where jitter silently collapses to 0.
  let mut s = ReconnectStrategy::with_rand(SeededRand::new(0xABCD_1234));
  let mut capped_delays = Vec::new();
  for _ in 0..MAX_RECONNECT_ATTEMPTS {
    if let Some(d) = s.next_delay() {
      // Only collect delays from the tier that is already capped so
      // we know the expected baseline (MAX_DELAY ± 10%).
      let max_ms = MAX_DELAY.as_millis() as u64;
      if d.as_millis() as u64 > max_ms * 80 / 100 {
        capped_delays.push(d.as_millis() as u64);
      }
    }
  }
  assert!(capped_delays.len() >= 3);
  let unique: std::collections::HashSet<_> = capped_delays.iter().collect();
  assert!(
    unique.len() > 1,
    "Seeded RNG should produce at least two distinct capped delays, got {:?}",
    capped_delays
  );
}
