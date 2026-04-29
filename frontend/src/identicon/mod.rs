//! Identicon avatar generator.
//!
//! Generates deterministic SVG-based identicon avatars from usernames.
//! Uses a simple hash of the username to produce a 5×5 symmetric grid
//! with a unique color derived from the hash.
//!
//! ## Hash Algorithm Choice (Issue-12)
//!
//! Req 10.6 suggests SHA-256 as the default hash. This implementation
//! uses **FNV-1a** (dual-hash variant) instead, for two reasons:
//!
//! 1. **WASM bundle size**: Including a SHA-256 implementation (e.g.
//!    `sha2` crate) adds ~15–30 KB to the compiled WASM binary. FNV-1a
//!    is implemented in ~10 lines and adds zero dependency overhead.
//! 2. **Collision risk is acceptable**: The identicon grid is only 5×5
//!    (15 independent bits for the left half) with an 18-entry color
//!    palette. Even a perfect hash would produce visually similar
//!    identicons at scale. FNV-1a provides sufficient distribution for
//!    this use case.
//!
//! If cryptographic uniqueness becomes a requirement (e.g. for avatar
//! fingerprint verification), replace `identicon_hash` with `sha2::Sha256`.

/// Color palette for identicon generation (HSL-based, visually distinct).
///
/// Each entry is a hue value in degrees. Saturation and lightness are
/// computed to produce pleasing colors.
const HUE_PALETTE: [u16; 18] = [
  0, 20, 40, 55, 120, 150, 170, 190, 210, 225, 245, 260, 280, 300, 320, 340, 15, 165,
];

/// Grid size for the identicon (5×5, but only left half + center need to be specified).
const GRID_SIZE: usize = 5;

/// Generate a deterministic SVG identicon for a given username.
///
/// The identicon is a 5×5 symmetric grid of colored squares on a
/// colored background. The pattern and colors are derived from a
/// simple hash of the input string.
///
/// Returns an SVG string that can be used as an `<img>` `src` (via data URI)
/// or rendered inline.
#[must_use]
pub fn generate_identicon_svg(username: &str) -> String {
  let hash = identicon_hash(username);
  let foreground_hue = HUE_PALETTE[(hash[0] as usize) % HUE_PALETTE.len()];
  let background_hue = (foreground_hue + 180) % 360;

  let foreground = format!("hsl({}, 65%, 55%)", foreground_hue);
  let background = format!("hsl({}, 30%, 92%)", background_hue);

  // Generate the grid pattern from hash bytes.
  // Only the left 3 columns need to be specified (columns 0,1,2);
  // columns 3,4 are mirrors of columns 1,0.
  let mut grid = [[false; GRID_SIZE]; GRID_SIZE];
  let mut byte_idx = 1;
  for row in &mut grid {
    for (_col, cell) in row.iter_mut().enumerate().take(3) {
      *cell = (hash[byte_idx % hash.len()] & 1) == 1;
      byte_idx += 1;
    }
    // Mirror: col 3 = col 1, col 4 = col 0
    row[3] = row[1];
    row[4] = row[0];
  }

  let cell_size: usize = 80;
  let svg_size = cell_size * GRID_SIZE;

  let mut rects = String::new();
  for (row_idx, row) in grid.iter().enumerate() {
    for (col_idx, &filled) in row.iter().enumerate() {
      if filled {
        let x = col_idx * cell_size;
        let y = row_idx * cell_size;
        rects.push_str(&format!(
          r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" />"#,
          x,
          y,
          cell_size,
          cell_size,
          cell_size / 5
        ));
      }
    }
  }

  let radius = svg_size / 5;
  format!(
    r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_size} {svg_size}" width="{svg_size}" height="{svg_size}">
  <rect width="{svg_size}" height="{svg_size}" fill="{background}" rx="{radius}" />
  <g fill="{foreground}">{rects}</g>
</svg>"#
  )
}

/// Generate a data URI for an identicon SVG.
///
/// This can be used directly as the `src` attribute of an `<img>` element.
///
/// ## Caching (Opt-4.3)
///
/// The data URI is deterministic in `username`, but previously every
/// render of every row / modal / panel re-encoded the SVG. A
/// thread-local `HashMap` now memoises the result so repeat lookups
/// cost a single hash instead of hundreds of `format!` + `replace`
/// allocations. `CACHE_CAPACITY` is a soft upper bound: once crossed
/// the cache is cleared wholesale (rather than pulling in a full LRU
/// implementation and its dependency). For typical discovery flows
/// (< 200 online users + blacklist + modals) the cache stays well
/// below that bound so the clear path never fires.
#[must_use]
pub fn generate_identicon_data_uri(username: &str) -> String {
  /// Soft cap on the cache to prevent unbounded growth across a long
  /// session. 1024 entries × ~1.5 KB/entry ≈ 1.5 MB, which is an
  /// acceptable worst case for a CSR app.
  const CACHE_CAPACITY: usize = 1024;

  thread_local! {
    static CACHE: std::cell::RefCell<std::collections::HashMap<String, String>> =
      std::cell::RefCell::new(std::collections::HashMap::new());
  }

  CACHE.with(|cell| {
    if let Some(cached) = cell.borrow().get(username) {
      return cached.clone();
    }

    let svg = generate_identicon_svg(username);
    let uri = format!("data:image/svg+xml;charset=utf-8,{}", url_encode_svg(&svg));

    let mut map = cell.borrow_mut();
    if map.len() >= CACHE_CAPACITY {
      map.clear();
    }
    map.insert(username.to_owned(), uri.clone());
    uri
  })
}

/// Simple URL encoding for SVG data URIs.
///
/// Only encodes characters that need escaping in a data URI.
fn url_encode_svg(svg: &str) -> String {
  svg
    .replace('%', "%25")
    .replace('#', "%23")
    .replace('"', "'")
    .replace('<', "%3C")
    .replace('>', "%3E")
    .replace('&', "%26")
    .replace(['\n', '\r'], "")
}

/// Deterministic hash function for identicon generation.
///
/// Produces an 8-byte array suitable for deriving grid patterns and colors.
/// This is NOT a cryptographic hash — it's only used for visual consistency.
/// Renamed from `simple_hash` to make the limited scope explicit (Review-L3).
fn identicon_hash(input: &str) -> [u8; 8] {
  let mut hash1: u32 = 0x811c_9dc5; // FNV-1a offset basis
  let mut hash2: u32 = 0xc1bd_ceee; // Second hash for more entropy

  for byte in input.bytes() {
    hash1 ^= u32::from(byte);
    hash1 = hash1.wrapping_mul(0x0100_0193); // FNV-1a prime

    hash2 = hash2.wrapping_mul(31).wrapping_add(u32::from(byte));
  }

  // Mix the two hashes together for more bit variety
  let combined = hash1 ^ hash2.wrapping_mul(0x9e37_79b9);

  let mut result = [0u8; 8];
  result[..4].copy_from_slice(&hash1.to_le_bytes());
  result[4..].copy_from_slice(&combined.to_le_bytes());

  result
}

#[cfg(test)]
mod tests;
