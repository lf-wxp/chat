//! Full-text search over persisted messages.
//!
//! Strategy:
//!
//! * For small stores (< [`INVERTED_INDEX_THRESHOLD`] total messages)
//!   we run a paged full scan. Pages of [`SEARCH_BATCH_SIZE`] records
//!   are fetched from IndexedDB and filtered in-memory. This keeps
//!   memory pressure bounded regardless of total store size while
//!   delivering ~O(hits) end-to-end latency for the common case.
//! * For large stores the caller should pre-build an inverted index
//!   using [`build_inverted_index`]. The index maps tokens to posting
//!   lists; queries become O(hits) rather than O(total).
//!
//! Scoring is deliberately lightweight: we count token hits and
//! weight by recency (Req 7.6 "Relevance Sorting"). Phrase matching
//! and fuzzy matching are out of scope for Task 17.

use crate::persistence::record::{ContentRecord, MessageRecord};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Minimum token length — tokens shorter than this are ignored so
/// stop words like "a", "i", "的" don't bloat the index.
const MIN_TOKEN_LEN: usize = 2;

/// Weight applied per token match.
const HIT_WEIGHT: f64 = 1.0;

/// Weight applied per distinct matching token (boost for messages
/// that hit every search term).
const DISTINCT_TOKEN_WEIGHT: f64 = 2.0;

/// Decay used for recency scoring. A message from 1 day ago scores
/// approximately `e^{-1}` lower than a message from the current
/// instant.
const RECENCY_HALF_LIFE_MS: f64 = 86_400_000.0;

/// Minimum recency factor to prevent very old messages from being
/// completely buried (they still score at least 10% of a fresh
/// message's recency weight).
const MIN_RECENCY_FACTOR: f64 = 0.1;

/// Search scope — global or scoped to one conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchScope {
  /// Search every persisted conversation.
  Global,
  /// Search only within the given conversation key.
  Conversation(String),
}

/// A search query (Req 7.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
  /// Raw user input — tokenised in [`tokenise`].
  pub raw: String,
  /// Search scope.
  pub scope: SearchScope,
  /// Maximum number of hits to return.
  pub limit: usize,
  /// Number of top hits to skip (for "Load more" pagination).
  pub offset: usize,
}

/// A single search hit with highlighting metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
  /// Persisted record.
  pub record: MessageRecord,
  /// Relevance score (higher = more relevant). Used for sorting.
  pub score: f64,
  /// Zero-based character offsets of token matches in the plain-text
  /// representation of the message body. The UI wraps these ranges
  /// with `<mark>` tags.
  pub highlights: Vec<(usize, usize)>,
}

/// Result of a search query.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
  /// Ranked hits (highest score first).
  pub hits: Vec<SearchHit>,
  /// Total number of messages inspected. Useful to display
  /// "Searched N messages" in the UI.
  pub scanned: usize,
}

/// Rank + filter `records` against `query` in memory.
///
/// This is the core scoring routine used by both the full-scan and
/// inverted-index search paths. It's side-effect free so it doubles
/// as the native test target.
#[must_use]
pub fn score_records(records: &[MessageRecord], query: &SearchQuery, now_ms: i64) -> SearchResult {
  let tokens = tokenise(&query.raw);
  if tokens.is_empty() {
    return SearchResult {
      hits: Vec::new(),
      scanned: records.len(),
    };
  }
  let mut scored: Vec<SearchHit> = Vec::new();
  for rec in records {
    if let SearchScope::Conversation(conv) = &query.scope
      && rec.conversation != *conv
    {
      continue;
    }
    let Some(body) = extract_body(&rec.content) else {
      continue;
    };
    let body_lower = body.to_lowercase();
    let mut score = 0.0;
    let mut highlights: Vec<(usize, usize)> = Vec::new();
    let mut distinct_hits: HashSet<&str> = HashSet::new();
    for token in &tokens {
      let mut search_start = 0;
      while let Some(offset) = body_lower[search_start..].find(token.as_str()) {
        let abs = search_start + offset;
        highlights.push((abs, abs + token.len()));
        score += HIT_WEIGHT;
        distinct_hits.insert(token.as_str());
        search_start = abs + token.len();
      }
    }
    if highlights.is_empty() {
      continue;
    }
    score += distinct_hits.len() as f64 * DISTINCT_TOKEN_WEIGHT;
    // Recency decay: `exp(-age / half_life)`.
    let age_ms = (now_ms - rec.timestamp_ms).max(0) as f64;
    let recency = (-age_ms / RECENCY_HALF_LIFE_MS).exp();
    score *= recency.max(MIN_RECENCY_FACTOR);
    // Deduplicate / sort highlight ranges so the UI can wrap them
    // linearly.
    merge_highlights(&mut highlights);
    scored.push(SearchHit {
      record: rec.clone(),
      score,
      highlights,
    });
  }
  scored.sort_by(|a, b| {
    b.score
      .partial_cmp(&a.score)
      .unwrap_or(std::cmp::Ordering::Equal)
  });
  if query.offset > 0 {
    scored = scored.into_iter().skip(query.offset).collect();
  }
  if query.limit > 0 {
    scored.truncate(query.limit);
  }
  SearchResult {
    hits: scored,
    scanned: records.len(),
  }
}

/// Tokenise `input` into normalised lowercase tokens suitable for
/// substring matching.
///
/// For ASCII / Latin text, tokens are split on non-alphanumeric chars
/// and short tokens (< `MIN_TOKEN_LEN` chars) are discarded.
///
/// For CJK characters (Chinese / Japanese / Korean), we emit overlapping
/// bigrams so that a query "你好" can match a record containing "你好世界"
/// through the inverted index path (not just substring search).
#[must_use]
pub fn tokenise(input: &str) -> Vec<String> {
  let lower = input.to_lowercase();
  let mut tokens = Vec::new();

  // Phase 1: standard split for non-CJK sequences.
  for segment in lower.split(|c: char| !c.is_alphanumeric()) {
    let chars: Vec<char> = segment.chars().collect();
    if chars.is_empty() {
      continue;
    }

    // Check if the segment is predominantly CJK.
    let cjk_count = chars.iter().filter(|c| is_cjk(**c)).count();
    if cjk_count > 0 {
      // Emit the full segment as one token (for exact match).
      if chars.len() >= MIN_TOKEN_LEN {
        tokens.push(segment.to_string());
      }
      // Emit overlapping bigrams for partial matching in the
      // inverted index path.
      if chars.len() >= 2 {
        for window in chars.windows(2) {
          let bigram: String = window.iter().collect();
          tokens.push(bigram);
        }
      }
      // Also emit individual CJK characters so single-char queries
      // still work through the index.
      for &ch in &chars {
        if is_cjk(ch) {
          tokens.push(ch.to_string());
        }
      }
    } else if chars.len() >= MIN_TOKEN_LEN {
      tokens.push(segment.to_string());
    }
  }

  // Deduplicate while preserving first-occurrence order so scoring
  // remains deterministic (R4 fix).
  let mut seen = HashSet::with_capacity(tokens.len());
  tokens.retain(|t| seen.insert(t.clone()));

  tokens
}

/// Returns `true` when `c` falls in a CJK Unified Ideograph range or
/// common CJK punctuation block.
fn is_cjk(c: char) -> bool {
  matches!(c,
    '\u{4E00}'..='\u{9FFF}'    // CJK Unified Ideographs
    | '\u{3400}'..='\u{4DBF}'  // CJK Unified Extension A
    | '\u{F900}'..='\u{FAFF}'  // CJK Compatibility Ideographs
    | '\u{3040}'..='\u{309F}'  // Hiragana
    | '\u{30A0}'..='\u{30FF}'  // Katakana
    | '\u{AC00}'..='\u{D7AF}'  // Hangul Syllables
  )
}

/// Lightweight inverted index mapping tokens → sorted postings.
///
/// Kept in memory; rebuilt on startup when the store grows past
/// [`crate::persistence::schema::INVERTED_INDEX_THRESHOLD`]. Rebuild
/// cost is `O(N)` and runs in an idle callback so UI responsiveness
/// is preserved.
#[derive(Debug, Default, Clone)]
pub struct InvertedIndex {
  /// token → list of record ids that contain it.
  pub(crate) postings: HashMap<String, Vec<String>>,
  /// message_id → conversation key (cheap lookup so the scorer can
  /// filter by scope without touching IndexedDB).
  pub(crate) conv_of: HashMap<String, String>,
  /// Total number of messages indexed.
  pub(crate) size: usize,
}

impl InvertedIndex {
  /// Total message count.
  #[must_use]
  pub fn len(&self) -> usize {
    self.size
  }

  /// Whether the index is empty.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.size == 0
  }

  /// Return a deduplicated list of record ids that contain every
  /// token in `query`. Empty queries return `None`.
  #[must_use]
  pub fn candidates(&self, query: &SearchQuery) -> Option<BTreeMap<String, usize>> {
    let tokens = tokenise(&query.raw);
    if tokens.is_empty() {
      return None;
    }
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for tok in &tokens {
      let Some(postings) = self.postings.get(tok) else {
        continue;
      };
      for id in postings {
        *counts.entry(id.as_str()).or_default() += 1;
      }
    }
    let mut filtered: BTreeMap<String, usize> = BTreeMap::new();
    for (id, hits) in counts {
      if let SearchScope::Conversation(c) = &query.scope
        && self.conv_of.get(id).map(String::as_str) != Some(c.as_str())
      {
        continue;
      }
      filtered.insert(id.to_string(), hits);
    }
    Some(filtered)
  }
}

/// Build an inverted index from a slice of records.
#[must_use]
pub fn build_inverted_index(records: &[MessageRecord]) -> InvertedIndex {
  let mut idx = InvertedIndex::default();
  extend_inverted_index(&mut idx, records);
  idx
}

/// Incrementally extend an existing inverted index with additional
/// records. Used by the streaming index rebuild path so the full
/// corpus need not be held in memory at once (BUG-5 / OOM fix).
///
/// Duplicate `message_id`s are silently skipped so that overlapping
/// paging batches (e.g. same `timestamp_ms` on the boundary) do not
/// corrupt the index.
pub fn extend_inverted_index(idx: &mut InvertedIndex, records: &[MessageRecord]) {
  let mut added = 0;
  for rec in records {
    if idx.conv_of.contains_key(&rec.message_id) {
      continue;
    }
    idx
      .conv_of
      .insert(rec.message_id.clone(), rec.conversation.clone());
    let Some(body) = extract_body(&rec.content) else {
      continue;
    };
    let tokens = tokenise(&body);
    let mut seen: HashSet<String> = HashSet::new();
    for tok in tokens {
      if seen.insert(tok.clone()) {
        idx
          .postings
          .entry(tok)
          .or_default()
          .push(rec.message_id.clone());
      }
    }
    added += 1;
  }
  idx.size += added;
}

/// Extract the searchable plaintext body of a record. Returns `None`
/// for content types (voice / image / sticker / file / revoked) that
/// carry no useful textual payload (`File` is searchable by filename,
/// which we surface here).
#[must_use]
pub fn extract_body(content: &ContentRecord) -> Option<String> {
  match content {
    ContentRecord::Text { text } => Some(text.clone()),
    ContentRecord::Forwarded { text, .. } => Some(text.clone()),
    ContentRecord::File { filename, .. } => Some(filename.clone()),
    ContentRecord::Sticker { .. }
    | ContentRecord::Voice { .. }
    | ContentRecord::Image { .. }
    | ContentRecord::Revoked => None,
  }
}

/// Merge overlapping / adjacent highlight spans so the UI does not
/// emit nested `<mark>` tags.
///
/// # Arguments
/// * `spans` - Mutable vector of `(start, end)` character offset pairs.
///   Modified in place to contain non-overlapping, sorted ranges.
///
/// # Example
/// ```ignore
/// let mut spans = vec![(0, 5), (3, 8), (12, 15)];
/// merge_highlights(&mut spans);
/// assert_eq!(spans, vec![(0, 8), (12, 15)]);
/// ```
fn merge_highlights(spans: &mut Vec<(usize, usize)>) {
  if spans.len() <= 1 {
    return;
  }
  spans.sort_by_key(|&(s, _)| s);
  let mut merged: Vec<(usize, usize)> = Vec::with_capacity(spans.len());
  for &(s, e) in spans.iter() {
    if let Some(last) = merged.last_mut()
      && s <= last.1
    {
      last.1 = last.1.max(e);
    } else {
      merged.push((s, e));
    }
  }
  *spans = merged;
}

// ── Paged full-scan search (WASM only) ────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::full_scan_search;

#[cfg(test)]
mod tests;
