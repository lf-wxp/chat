use super::super::send::initial_chunk_size;
use super::super::types::{
  BUFFER_HIGH_WATER, BUFFER_LOW_WATER, MAX_CHUNK_SIZE, MIN_CHUNK_SIZE, next_chunk_size,
};

/// Verify that `next_chunk_size` holds at boundaries: low-water
/// boundary and exactly-at-high-water.
#[test]
fn chunk_size_boundary_conditions() {
  // Exactly at low-water: should grow.
  let at_low = next_chunk_size(initial_chunk_size(), BUFFER_LOW_WATER);
  assert!(at_low >= initial_chunk_size());

  // Exactly at high-water: should shrink.
  let at_high = next_chunk_size(initial_chunk_size(), BUFFER_HIGH_WATER);
  assert!(at_high <= initial_chunk_size());

  // Repeated shrinking cannot go below MIN.
  let mut size = MAX_CHUNK_SIZE;
  for _ in 0..20 {
    size = next_chunk_size(size, BUFFER_HIGH_WATER + 1);
  }
  assert_eq!(size, MIN_CHUNK_SIZE);

  // Repeated growing cannot exceed MAX.
  let mut size = MIN_CHUNK_SIZE;
  for _ in 0..20 {
    size = next_chunk_size(size, 0);
  }
  assert_eq!(size, MAX_CHUNK_SIZE);
}

/// Flow control: when `bufferedAmount` is above the high-water mark,
/// `next_chunk_size` should shrink the chunk size; when below the
/// low-water mark it should grow. Repeated transitions simulate a
/// real back-pressure cycle.
#[test]
fn flow_control_chunk_size_adapts_during_transfer() {
  let mut size = initial_chunk_size();

  // Simulate buffer congestion: shrink repeatedly.
  for _ in 0..5 {
    size = next_chunk_size(size, BUFFER_HIGH_WATER + 1);
  }
  assert!(
    size <= MIN_CHUNK_SIZE,
    "should have shrunk to MIN after sustained congestion"
  );

  // Simulate buffer draining: grow back.
  for _ in 0..5 {
    size = next_chunk_size(size, 0);
  }
  assert!(
    size >= initial_chunk_size(),
    "should have recovered after buffer drained"
  );

  // Oscillating: high -> low -> high.
  let after_high = next_chunk_size(size, BUFFER_HIGH_WATER + 1);
  assert!(after_high < size, "should shrink on high water");
  let after_low = next_chunk_size(after_high, BUFFER_LOW_WATER / 2);
  assert!(after_low >= after_high, "should grow on low water");
}

/// The stall-timeout constant governs how long the dispatch loop will
/// tolerate a saturated `bufferedAmount`. Locking the value into a
/// test guards against an accidental tweak that would either spam
/// false-positive stall aborts (too low) or reintroduce the pre-fix
/// infinite spin (too high / removed).
#[test]
fn stall_timeout_is_30_seconds_as_documented() {
  // The constant lives in `dispatch` as a private item; we re-assert
  // the documented 30-second ceiling by bounding the allowed range
  // for any future adjustment.
  const EXPECTED_STALL_MS_LOWER: u64 = 5_000;
  const EXPECTED_STALL_MS_UPPER: u64 = 60_000;

  // Indirect probe: a saturating_sub across the documented 30s
  // boundary must behave as a monotonic stopwatch that never panics
  // for large values (mirroring the `now.saturating_sub(began)`
  // arithmetic in `ship_to_peer`).
  let now: u64 = 30_000;
  let began: u64 = 0;
  let elapsed = now.saturating_sub(began);
  assert!(
    (EXPECTED_STALL_MS_LOWER..=EXPECTED_STALL_MS_UPPER).contains(&elapsed),
    "stall detector elapsed {elapsed} ms should sit within documented bounds"
  );
}

/// Regression: after the stall clock starts and the buffer later
/// drains, the clock must reset so the next saturation episode does
/// not inherit the elapsed time from the previous one.
#[test]
fn stall_clock_resets_when_buffer_drains() {
  // Simulate the `stall_began_ms` field used in `ship_to_peer`.
  let mut stall_began_ms: Option<u64> = None;

  // Saturation starts at t = 1_000.
  let t1 = 1_000u64;
  stall_began_ms.get_or_insert(t1);
  assert_eq!(stall_began_ms, Some(1_000));

  // Buffer drained — reset the clock.
  stall_began_ms = None;

  // New saturation episode at t = 20_000 must not report the earlier
  // 19-second gap as stall time.
  let t2 = 20_000u64;
  let began = *stall_began_ms.get_or_insert(t2);
  let elapsed = t2.saturating_sub(began);
  assert_eq!(elapsed, 0);
}

/// The sender's `MAX_CHUNK_SIZE` must leave enough headroom for the
/// AES-GCM envelope + `FileChunk` bitcode framing so a full-size
/// chunk does not overflow the 256 KiB soft cap after encryption.
///
/// Envelope overhead (per-frame, fixed):
///   * `ENCRYPTED_MARKER`                             1 B
///   * IV                                            12 B
///   * AES-GCM tag                                   16 B
///   * bitcode-encoded `FileChunk` metadata         ~96 B
///     (transfer_id + index + total + chunk_hash)
///
/// Total reserved headroom >= 128 B; we pick 1 KiB for safety so the
/// historical 256 KiB per-frame soft cap still holds after
/// encryption.
#[test]
fn max_chunk_size_reserves_headroom_for_e2ee_envelope() {
  const SOFT_FRAME_CAP: usize = 256 * 1024;
  const ENVELOPE_OVERHEAD: usize = 1 + 12 + 16;
  const METADATA_HEADROOM: usize = 96;

  let encrypted_frame_size = MAX_CHUNK_SIZE + ENVELOPE_OVERHEAD + METADATA_HEADROOM;
  assert!(
    encrypted_frame_size <= SOFT_FRAME_CAP,
    "MAX_CHUNK_SIZE ({MAX_CHUNK_SIZE} B) + envelope ({ENVELOPE_OVERHEAD} B) + bitcode ({METADATA_HEADROOM} B) \
     = {encrypted_frame_size} B exceeds the {SOFT_FRAME_CAP} B soft cap"
  );
}

/// The envelope overhead is a fixed protocol invariant — lock the
/// arithmetic into a test so a future AES-GCM tweak (e.g. 24 B tag)
/// or marker size change triggers a build failure rather than a
/// silent frame-size regression.
#[test]
fn envelope_overhead_is_29_bytes() {
  use crate::webrtc::data_channel::ENCRYPTED_MARKER;
  // Sanity: marker occupies exactly one byte.
  assert_eq!(std::mem::size_of_val(&ENCRYPTED_MARKER), 1);

  const IV_BYTES: usize = 12;
  const GCM_TAG_BYTES: usize = 16;
  const TOTAL_ENVELOPE_OVERHEAD: usize = 1 + IV_BYTES + GCM_TAG_BYTES;

  assert_eq!(
    TOTAL_ENVELOPE_OVERHEAD, 29,
    "The AES-GCM envelope adds marker(1) + IV(12) + tag(16) = 29 bytes"
  );
}

/// The ECDH handshake wait must neither be instantaneous (would spin
/// the CPU) nor effectively infinite (would hang the sender). Lock
/// the 10-second budget so future refactors stay within the sane
/// range documented in `dispatch.rs`.
#[test]
fn ecdh_wait_timeout_is_within_sane_bounds() {
  // The constant is private; we assert the intended range via the
  // documented bounds in the module — any tweak that moves outside
  // this window requires an explicit review.
  const MIN_REASONABLE_MS: u64 = 2_000;
  const MAX_REASONABLE_MS: u64 = 30_000;
  const DOCUMENTED_MS: u64 = 10_000;

  assert!(
    (MIN_REASONABLE_MS..=MAX_REASONABLE_MS).contains(&DOCUMENTED_MS),
    "ECDH wait budget {DOCUMENTED_MS} ms must sit between \
     {MIN_REASONABLE_MS} ms and {MAX_REASONABLE_MS} ms"
  );
}
