//! JWT token decoding and expiry checking.
//!
//! Provides lightweight JWT payload decoding without signature verification
//! and pure-Rust expiry logic that can be unit-tested on native targets.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

/// Permitted clock skew between the browser and the issuing server,
/// expressed in seconds. Used only for `nbf` validation; `exp` is checked
/// exactly so we never cling to an already-expired token.
pub(crate) const JWT_CLOCK_SKEW_SECS: u64 = 60;

/// Decode the payload section of a JWT token.
///
/// Returns the decoded JSON payload string, or `None` if the token is
/// malformed or the decode fails. This is the WASM-only half of
/// [`is_jwt_expired`]; the pure-Rust expiry logic lives in
/// [`is_payload_expired`].
///
/// Uses the Rust `base64` crate with base64url (URL-safe, no padding)
/// instead of `window.atob()` so that non-ASCII string claims (e.g.
/// Unicode usernames) are decoded correctly as UTF-8 rather than being
/// corrupted by `atob()`'s Latin1 interpretation.
fn decode_jwt_payload(token: &str) -> Option<String> {
  let parts: Vec<&str> = token.split('.').collect();
  if parts.len() != 3 {
    return None;
  }
  let payload_b64 = parts[1];

  let bytes = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
  String::from_utf8(bytes).ok()
}

/// Check whether a decoded JWT payload has expired.
///
/// Pure Rust — does not depend on browser APIs and can be fully unit
/// tested on native targets (Issue-3 fix).
///
/// # Arguments
///
/// * `payload` — The decoded JSON payload string (e.g. from
///   [`decode_jwt_payload`]).
/// * `now_secs` — The current Unix timestamp in seconds.
///
/// # Logic
///
/// - Missing `exp` → treated as non-expiring (server has final say).
/// - Non-numeric `exp` → treated as expired (fail-safe).
/// - `nbf` claim is honoured with a [`JWT_CLOCK_SKEW_SECS`]-second grace
///   window to tolerate minor client/server clock drift (P1-3 fix).
pub(crate) fn is_payload_expired(payload: &str, now_secs: u64) -> bool {
  let parsed: serde_json::Value = match serde_json::from_str(payload) {
    Ok(v) => v,
    Err(_) => return true,
  };

  // Honour `nbf` (Not Before). A token whose `nbf` is in the future is
  // not yet valid; treat it as "expired" so the caller clears it. We
  // allow up to `JWT_CLOCK_SKEW_SECS` of client clock drift to avoid
  // rejecting tokens simply because the browser clock is a few seconds
  // ahead of the server (P1-3 fix).
  if let Some(nbf_val) = parsed.get("nbf")
    && let Some(nbf_secs) = nbf_val
      .as_u64()
      .or_else(|| nbf_val.as_f64().map(|f| f as u64))
    && nbf_secs > now_secs.saturating_add(JWT_CLOCK_SKEW_SECS)
  {
    return true;
  }

  // Only accept numeric `exp` claims. A string, boolean, or missing-but-
  // typed-wrong value is treated as expired so we never forward a token
  // the server would certainly reject (R2-Issue-9 hardening).
  match parsed.get("exp") {
    Some(v) => match v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)) {
      Some(exp_secs) => exp_secs <= now_secs,
      // Non-numeric `exp` — fail-safe: treat as expired.
      None => true,
    },
    // No exp claim — treat as non-expiring (server will validate).
    None => false,
  }
}

/// Check whether a JWT token has expired by decoding the payload `exp` claim.
///
/// Performs a lightweight base64 decode of the JWT payload without verifying
/// the signature. Returns `true` if the token is expired or cannot be parsed
/// (fail-safe: treat unparseable tokens as expired so the caller clears them).
///
/// # Server contract
///
/// This function assumes the server *always* issues tokens with a valid
/// numeric `exp` claim. If the backend ever starts issuing perpetual
/// tokens (no `exp`), the `None => false` branch below will treat such
/// tokens as non-expiring. Review this function together with
/// `server/src/auth/token.rs` whenever the token policy changes.
///
/// # WASM-only
///
/// This function depends on `web_sys::window().atob()` for base64 decoding
/// and will return `true` (treat as expired) on non-WASM targets where the
/// browser `window` object is unavailable. Do **not** call this from native
/// code paths — it is only meaningful inside a WASM browser context.
#[doc(hidden)]
pub fn is_jwt_expired(token: &str) -> bool {
  let Some(decoded) = decode_jwt_payload(token) else {
    return true;
  };
  let now_secs = (js_sys::Date::now() / 1000.0) as u64;
  is_payload_expired(&decoded, now_secs)
}
