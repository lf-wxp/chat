//! Retention & quota enforcement.
//!
//! Two entry points:
//!
//! * [`sweep_retention`] — delete every message older than the
//!   configured policy window (default 72 h, Req 11.4). Runs on a
//!   1-minute timer and once on startup.
//! * [`cleanup_on_quota_exceeded`] — triggered when a write fails
//!   with `QuotaExceededError`. First trims the retention window to
//!   one day, and if that still doesn't fit, drops the oldest 10 %
//!   of messages.
//!
//! Pure-data helpers (cutoff calculation, chunk size selection) live
//! here too and are unit-tested natively.

use crate::persistence::record::RetentionPolicy;

/// Minimum retention window used as the fallback trimming target
/// when the user hits `QuotaExceededError`. Corresponds to the
/// smallest configurable retention (24 h).
pub const MIN_RETENTION_MS: i64 = 24 * 60 * 60 * 1_000;

/// When the normal retention sweep doesn't free enough space, we
/// drop this fraction of the oldest messages as a secondary
/// fallback (Req 11.4, "Automatic cleanup of oldest messages").
pub const QUOTA_FALLBACK_FRACTION: f64 = 0.10;

/// Compute the cut-off timestamp for a retention sweep.
///
/// Messages with `timestamp_ms <= cutoff` are candidates for deletion.
#[must_use]
pub const fn retention_cutoff(now_ms: i64, policy: RetentionPolicy) -> i64 {
  now_ms.saturating_sub(policy.as_ms())
}

/// Return the number of oldest messages to drop when normal sweep
/// fails. The caller passes the current message count; the result
/// is the ceiling of `count * 0.10`, clamped to `[1, count]`.
/// Returns 0 when `total_messages` is 0 (nothing to delete).
#[must_use]
pub fn quota_fallback_count(total_messages: usize) -> usize {
  if total_messages == 0 {
    return 0;
  }
  let base = (total_messages as f64 * QUOTA_FALLBACK_FRACTION).ceil() as usize;
  base.clamp(1, total_messages)
}

/// Pretty-print a storage byte count for the UI banner.
///
/// Automatically selects the appropriate unit (B, KB, MB, GB) based
/// on the magnitude of the input value.
///
/// # Examples
/// ```ignore
/// assert_eq!(format_bytes(1024), "1.00 KB");
/// assert_eq!(format_bytes(5 * 1024 * 1024), "5.00 MB");
/// ```
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
  const KB: u64 = 1024;
  const MB: u64 = KB * 1024;
  const GB: u64 = MB * 1024;
  if bytes >= GB {
    format!("{:.2} GB", bytes as f64 / GB as f64)
  } else if bytes >= MB {
    format!("{:.2} MB", bytes as f64 / MB as f64)
  } else if bytes >= KB {
    format!("{:.2} KB", bytes as f64 / KB as f64)
  } else {
    format!("{bytes} B")
  }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
  use super::{MIN_RETENTION_MS, QUOTA_FALLBACK_FRACTION, quota_fallback_count, retention_cutoff};
  use crate::persistence::idb::{IdbResult, estimate_storage};
  use crate::persistence::record::RetentionPolicy;
  use crate::persistence::store::{count_all, delete_older_than, delete_oldest};
  use web_sys::IdbDatabase;

  /// Quota fill threshold above which we preemptively trim (0..=1.0).
  const QUOTA_WARNING_FRACTION: f64 = 0.90;

  /// Run a retention sweep. Returns the number of records deleted.
  pub async fn sweep_retention(
    db: &IdbDatabase,
    policy: RetentionPolicy,
    now_ms: i64,
  ) -> IdbResult<usize> {
    let cutoff = retention_cutoff(now_ms, policy);
    if cutoff <= 0 {
      return Ok(0);
    }
    delete_older_than(db, cutoff).await
  }

  /// Cleanup routine invoked when IndexedDB reports
  /// `QuotaExceededError`. Runs two trimming passes in order.
  pub async fn cleanup_on_quota_exceeded(db: &IdbDatabase, now_ms: i64) -> IdbResult<usize> {
    // Pass 1: trim retention to the minimum window.
    let cutoff = now_ms.saturating_sub(MIN_RETENTION_MS);
    let mut total = delete_older_than(db, cutoff).await.unwrap_or(0);

    // Re-check quota; if still tight, drop the oldest 10 %.
    let tight = match estimate_storage().await {
      Ok((usage, quota)) if quota > 0 => (usage as f64) / (quota as f64) >= QUOTA_WARNING_FRACTION,
      _ => true, // If we can't measure, assume still tight.
    };
    if tight {
      let total_messages = count_all(db).await.unwrap_or(0);
      let drop = quota_fallback_count(total_messages);
      total = total.saturating_add(delete_oldest(db, drop).await.unwrap_or(0));
    }
    Ok(total)
  }

  /// Whether storage usage is close enough to quota that a retention
  /// sweep should run proactively.
  pub async fn is_near_quota() -> bool {
    match estimate_storage().await {
      Ok((usage, quota)) if quota > 0 => (usage as f64) / (quota as f64) >= QUOTA_WARNING_FRACTION,
      _ => false,
    }
  }

  // Silence dead-code warnings from the module-level constant when
  // compiled without callers in this translation unit.
  const _: f64 = QUOTA_FALLBACK_FRACTION;
}

#[cfg(target_arch = "wasm32")]
pub use wasm::{cleanup_on_quota_exceeded, is_near_quota, sweep_retention};

#[cfg(test)]
mod tests;
