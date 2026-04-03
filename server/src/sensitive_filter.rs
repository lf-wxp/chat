//! Advanced sensitive word filter with Aho-Corasick algorithm.
//!
//! This module provides efficient multi-pattern string matching with:
//! - O(n) time complexity using Aho-Corasick automaton
//! - Configurable sensitivity levels (High/Medium/Low)
//! - External word list support via JSON files
//! - Real-time statistics and audit logging
//! - Hot-reload capability for word list updates

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aho_corasick::AhoCorasick;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

// =============================================================================
// Sensitivity Levels
// =============================================================================

/// Sensitivity level for categorizing filtered content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SensitivityLevel {
  /// Highly prohibited content (immediate block, log event)
  High,
  /// Moderately sensitive content (filter with warning)
  Medium,
  /// Mildly sensitive content (filter without immediate action)
  Low,
}

impl Default for SensitivityLevel {
  fn default() -> Self {
    Self::Medium
  }
}

impl SensitivityLevel {
  /// Get the label for this sensitivity level.
  pub fn label(self) -> &'static str {
    match self {
      Self::High => "high",
      Self::Medium => "medium",
      Self::Low => "low",
    }
  }

  /// Get the replacement character for this level.
  pub fn mask_char(self) -> char {
    match self {
      Self::High => '█',
      Self::Medium => '▓',
      Self::Low => '░',
    }
  }
}

// =============================================================================
// Sensitive Word Entry
// =============================================================================

/// A single sensitive word entry with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveWordEntry {
  /// The word pattern (case-insensitive)
  pub word: String,
  /// Sensitivity level
  #[serde(default)]
  pub level: SensitivityLevel,
  /// Optional category (e.g., "profanity", "discrimination")
  #[serde(default)]
  pub category: Option<String>,
  /// Optional description for moderators
  #[serde(default)]
  pub description: Option<String>,
}

// =============================================================================
// Word List Configuration
// =============================================================================

/// Configuration for a word list loaded from file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordListConfig {
  /// Version of the word list
  pub version: String,
  /// When the word list was last updated
  #[serde(default = "Utc::now")]
  pub updated_at: DateTime<Utc>,
  /// List of sensitive words
  pub words: Vec<SensitiveWordEntry>,
}

// =============================================================================
// Filter Statistics
// =============================================================================

/// Statistics for filter operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilterStats {
  /// Total number of filter operations
  pub total_operations: u64,
  /// Number of operations that found sensitive content
  pub filtered_count: u64,
  /// Breakdown by sensitivity level
  pub by_level: HashMap<String, u64>,
  /// Breakdown by category
  pub by_category: HashMap<String, u64>,
  /// Last update timestamp
  pub last_updated: Option<DateTime<Utc>>,
}

// =============================================================================
// Filter Event
// =============================================================================

/// An audit event for sensitive word filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterEvent {
  /// Unique event ID
  pub id: String,
  /// When the event occurred
  pub timestamp: DateTime<Utc>,
  /// User ID (if available)
  pub user_id: Option<String>,
  /// Room ID (if available)
  pub room_id: Option<String>,
  /// The matched word
  pub matched_word: String,
  /// Sensitivity level
  pub level: SensitivityLevel,
  /// Category
  pub category: Option<String>,
  /// Context (surrounding text, truncated)
  pub context: String,
}

// =============================================================================
// Advanced Sensitive Word Filter
// =============================================================================

/// Advanced sensitive word filter using Aho-Corasick algorithm.
///
/// This filter provides:
/// - O(n) time complexity for multi-pattern matching
/// - Configurable sensitivity levels
/// - External word list support
/// - Statistics and audit logging
///
/// # Examples
///
/// ```ignore
/// use server::sensitive_filter::AdvancedSensitiveWordFilter;
///
/// let filter = AdvancedSensitiveWordFilter::with_builtin_words();
/// let result = filter.filter("some text");
/// assert!(!result.had_sensitive);
/// ```
#[derive(Clone)]
pub struct AdvancedSensitiveWordFilter {
  /// Inner state protected by RwLock for concurrent access
  inner: Arc<RwLock<FilterInner>>,
  /// Path to external word list file (if any)
  config_path: Option<PathBuf>,
}

/// Inner state of the filter.
struct FilterInner {
  /// Aho-Corasick automaton for pattern matching
  automaton: AhoCorasick,
  /// Word entries indexed by pattern ID
  word_entries: Vec<SensitiveWordEntry>,
  /// Statistics
  stats: FilterStats,
  /// Recent filter events (last 1000)
  recent_events: Vec<FilterEvent>,
  /// Configuration
  config: WordListConfig,
}

impl Default for AdvancedSensitiveWordFilter {
  fn default() -> Self {
    Self::with_builtin_words()
  }
}

impl AdvancedSensitiveWordFilter {
  /// Maximum number of recent events to keep.
  const MAX_RECENT_EVENTS: usize = 1000;

  /// Maximum context length for audit events.
  const MAX_CONTEXT_LENGTH: usize = 100;

  /// Create a new filter with built-in word list.
  pub fn with_builtin_words() -> Self {
    let config = Self::builtin_word_list();
    let inner = Self::build_inner(&config);
    Self {
      inner: Arc::new(RwLock::new(inner)),
      config_path: None,
    }
  }

  /// Create a new filter with external word list file.
  ///
  /// # Errors
  /// Returns an error if the file cannot be read or parsed.
  pub fn with_config_file<P: AsRef<Path>>(path: P) -> Result<Self, FilterError> {
    let path = path.as_ref();
    let config = Self::load_config(path)?;
    let inner = Self::build_inner(&config);
    Ok(Self {
      inner: Arc::new(RwLock::new(inner)),
      config_path: Some(path.to_path_buf()),
    })
  }

  /// Load configuration from a JSON file.
  fn load_config(path: &Path) -> Result<WordListConfig, FilterError> {
    let content = fs::read_to_string(path)
      .map_err(|e| FilterError::ConfigLoad(format!("Failed to read {}: {}", path.display(), e)))?;
    let config: WordListConfig = serde_json::from_str(&content)
      .map_err(|e| FilterError::ConfigLoad(format!("Failed to parse {}: {}", path.display(), e)))?;
    Ok(config)
  }

  /// Build the inner state from configuration.
  fn build_inner(config: &WordListConfig) -> FilterInner {
    let patterns: Vec<String> = config.words.iter().map(|w| w.word.to_lowercase()).collect();
    let automaton = AhoCorasick::new(&patterns).expect("Failed to build Aho-Corasick automaton");
    FilterInner {
      automaton,
      word_entries: config.words.clone(),
      stats: FilterStats::default(),
      recent_events: Vec::with_capacity(Self::MAX_RECENT_EVENTS),
      config: config.clone(),
    }
  }

  /// Get the built-in word list configuration.
  fn builtin_word_list() -> WordListConfig {
    let words = vec![
      // High sensitivity - discrimination and severe profanity
      SensitiveWordEntry {
        word: "nigger".to_string(),
        level: SensitivityLevel::High,
        category: Some("discrimination".to_string()),
        description: Some("Racial slur".to_string()),
      },
      SensitiveWordEntry {
        word: "faggot".to_string(),
        level: SensitivityLevel::High,
        category: Some("discrimination".to_string()),
        description: Some("Homophobic slur".to_string()),
      },
      // Medium sensitivity - common profanity
      SensitiveWordEntry {
        word: "fuck".to_string(),
        level: SensitivityLevel::Medium,
        category: Some("profanity".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "shit".to_string(),
        level: SensitivityLevel::Medium,
        category: Some("profanity".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "asshole".to_string(),
        level: SensitivityLevel::Medium,
        category: Some("profanity".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "bitch".to_string(),
        level: SensitivityLevel::Medium,
        category: Some("profanity".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "bastard".to_string(),
        level: SensitivityLevel::Medium,
        category: Some("profanity".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "dick".to_string(),
        level: SensitivityLevel::Medium,
        category: Some("profanity".to_string()),
        description: None,
      },
      // Low sensitivity - mild profanity
      SensitiveWordEntry {
        word: "damn".to_string(),
        level: SensitivityLevel::Low,
        category: Some("mild".to_string()),
        description: None,
      },
      // Chinese sensitive words
      SensitiveWordEntry {
        word: "操你妈".to_string(),
        level: SensitivityLevel::High,
        category: Some("profanity".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "草泥马".to_string(),
        level: SensitivityLevel::High,
        category: Some("profanity".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "傻逼".to_string(),
        level: SensitivityLevel::Medium,
        category: Some("profanity".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "他妈的".to_string(),
        level: SensitivityLevel::Medium,
        category: Some("profanity".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "妈的".to_string(),
        level: SensitivityLevel::Low,
        category: Some("mild".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "狗日的".to_string(),
        level: SensitivityLevel::Medium,
        category: Some("profanity".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "混蛋".to_string(),
        level: SensitivityLevel::Medium,
        category: Some("profanity".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "王八蛋".to_string(),
        level: SensitivityLevel::Medium,
        category: Some("profanity".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "去死".to_string(),
        level: SensitivityLevel::Medium,
        category: Some("threat".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "废物".to_string(),
        level: SensitivityLevel::Low,
        category: Some("insult".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "垃圾".to_string(),
        level: SensitivityLevel::Low,
        category: Some("insult".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "白痴".to_string(),
        level: SensitivityLevel::Low,
        category: Some("insult".to_string()),
        description: None,
      },
      SensitiveWordEntry {
        word: "脑残".to_string(),
        level: SensitivityLevel::Medium,
        category: Some("insult".to_string()),
        description: None,
      },
    ];

    WordListConfig {
      version: "1.0.0".to_string(),
      updated_at: Utc::now(),
      words,
    }
  }

  /// Reload the word list from the external file (if configured).
  ///
  /// # Errors
  /// Returns an error if the file cannot be read or parsed.
  pub fn reload(&self) -> Result<(), FilterError> {
    let path = self.config_path.as_ref().ok_or(FilterError::NoConfigFile)?;
    let config = Self::load_config(path)?;
    let new_inner = Self::build_inner(&config);

    let mut inner = self.inner.write();
    inner.automaton = new_inner.automaton;
    inner.word_entries = new_inner.word_entries;
    inner.config = new_inner.config;

    info!(
      "[SensitiveFilter] Reloaded word list from {}",
      path.display()
    );
    Ok(())
  }

  /// Check if text contains any sensitive words.
  ///
  /// Returns `true` if at least one sensitive word is found.
  #[must_use]
  pub fn contains_sensitive(&self, text: &str) -> bool {
    let inner = self.inner.read();
    let lower = text.to_lowercase();
    inner.automaton.is_match(&lower)
  }

  /// Find all sensitive words in the text.
  ///
  /// Returns a list of matches with their positions and metadata.
  #[must_use]
  pub fn find_sensitive(&self, text: &str) -> Vec<SensitiveMatch> {
    let inner = self.inner.read();
    let lower = text.to_lowercase();
    let chars: Vec<char> = text.chars().collect();
    let _lower_chars: Vec<char> = lower.chars().collect();

    let mut matches = Vec::new();
    for mat in inner.automaton.find_iter(&lower) {
      let pattern_id = mat.pattern();
      let entry = &inner.word_entries[pattern_id.as_usize()];

      // Convert byte positions to character positions
      let start_byte = mat.start();
      let end_byte = mat.end();
      let start_char = lower[..start_byte].chars().count();
      let end_char = lower[..end_byte].chars().count();

      let matched_text: String = chars[start_char..end_char].iter().collect();

      matches.push(SensitiveMatch {
        word: entry.word.clone(),
        matched_text,
        start: start_char,
        end: end_char,
        level: entry.level,
        category: entry.category.clone(),
      });
    }
    matches
  }

  /// Filter sensitive words in the text, replacing them with mask characters.
  ///
  /// Returns the filtered text and match information.
  #[must_use]
  pub fn filter(&self, text: &str) -> FilterResult {
    let matches = self.find_sensitive(text);

    if matches.is_empty() {
      return FilterResult {
        text: text.to_string(),
        matches: vec![],
        had_sensitive: false,
      };
    }

    let chars: Vec<char> = text.chars().collect();
    let mut masked = chars.clone();

    // Apply masks (in reverse order to preserve positions)
    for m in &matches {
      let mask_char = m.level.mask_char();
      for i in m.start..m.end {
        if i < masked.len() {
          masked[i] = mask_char;
        }
      }
    }

    FilterResult {
      text: masked.into_iter().collect(),
      matches,
      had_sensitive: true,
    }
  }

  /// Filter with audit logging.
  ///
  /// This records filter events for auditing purposes.
  pub fn filter_with_audit(
    &self,
    text: &str,
    user_id: Option<&str>,
    room_id: Option<&str>,
  ) -> FilterResult {
    let result = self.filter(text);

    // Always update total_operations
    {
      let mut inner = self.inner.write();
      inner.stats.total_operations += 1;
      inner.stats.last_updated = Some(Utc::now());
    }

    if result.had_sensitive {
      let mut inner = self.inner.write();

      // Update filtered count
      inner.stats.filtered_count += 1;

      // Record events
      for m in &result.matches {
        // Update level stats
        *inner
          .stats
          .by_level
          .entry(m.level.label().to_string())
          .or_insert(0) += 1;

        // Update category stats
        if let Some(cat) = &m.category {
          *inner.stats.by_category.entry(cat.clone()).or_insert(0) += 1;
        }

        // Create event
        let context = Self::truncate_context(text, Self::MAX_CONTEXT_LENGTH);
        let event = FilterEvent {
          id: nanoid::nanoid!(16),
          timestamp: Utc::now(),
          user_id: user_id.map(|s| s.to_string()),
          room_id: room_id.map(|s| s.to_string()),
          matched_word: m.word.clone(),
          level: m.level,
          category: m.category.clone(),
          context,
        };

        // Add to recent events
        if inner.recent_events.len() >= Self::MAX_RECENT_EVENTS {
          inner.recent_events.remove(0);
        }
        inner.recent_events.push(event);

        // Log based on level
        match m.level {
          SensitivityLevel::High => {
            warn!("[SensitiveFilter] HIGH: '{}' by user {:?}", m.word, user_id);
          }
          SensitivityLevel::Medium => {
            info!(
              "[SensitiveFilter] MEDIUM: '{}' by user {:?}",
              m.word, user_id
            );
          }
          SensitivityLevel::Low => {
            // Low level events are not logged individually
          }
        }
      }
    }

    result
  }

  /// Get current statistics.
  #[must_use]
  pub fn stats(&self) -> FilterStats {
    self.inner.read().stats.clone()
  }

  /// Get recent filter events.
  #[must_use]
  pub fn recent_events(&self, limit: usize) -> Vec<FilterEvent> {
    let inner = self.inner.read();
    let start = inner.recent_events.len().saturating_sub(limit);
    inner.recent_events[start..].to_vec()
  }

  /// Get the word list configuration.
  #[must_use]
  pub fn config(&self) -> WordListConfig {
    self.inner.read().config.clone()
  }

  /// Truncate context for audit events.
  fn truncate_context(text: &str, max_len: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_len {
      text.to_string()
    } else {
      let truncated: String = chars[..max_len.saturating_sub(1)].iter().collect();
      format!("{}…", truncated)
    }
  }
}

// =============================================================================
// Sensitive Match
// =============================================================================

/// A single sensitive word match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveMatch {
  /// The original word pattern
  pub word: String,
  /// The actual matched text (may differ in case)
  pub matched_text: String,
  /// Start position (character index)
  pub start: usize,
  /// End position (character index)
  pub end: usize,
  /// Sensitivity level
  pub level: SensitivityLevel,
  /// Category
  pub category: Option<String>,
}

// =============================================================================
// Filter Result
// =============================================================================

/// Result of a filter operation.
#[derive(Debug, Clone)]
pub struct FilterResult {
  /// The filtered text
  pub text: String,
  /// List of matches found
  pub matches: Vec<SensitiveMatch>,
  /// Whether any sensitive content was found
  pub had_sensitive: bool,
}

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during filter operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum FilterError {
  /// Failed to load configuration file
  #[error("Configuration load error: {0}")]
  ConfigLoad(String),

  /// No configuration file set
  #[error("No configuration file set")]
  NoConfigFile,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  fn create_test_filter() -> AdvancedSensitiveWordFilter {
    AdvancedSensitiveWordFilter::with_builtin_words()
  }

  #[test]
  fn test_contains_sensitive_positive() {
    let filter = create_test_filter();
    assert!(filter.contains_sensitive("you are a bastard"));
    assert!(filter.contains_sensitive("你真是个傻逼"));
  }

  #[test]
  fn test_contains_sensitive_negative() {
    let filter = create_test_filter();
    assert!(!filter.contains_sensitive("hello world"));
    assert!(!filter.contains_sensitive("你好世界"));
  }

  #[test]
  fn test_find_sensitive_multiple() {
    let filter = create_test_filter();
    let matches = filter.find_sensitive("you damn bastard");
    assert_eq!(matches.len(), 2);

    assert_eq!(matches[0].word, "damn");
    assert_eq!(matches[0].level, SensitivityLevel::Low);

    assert_eq!(matches[1].word, "bastard");
    assert_eq!(matches[1].level, SensitivityLevel::Medium);
  }

  #[test]
  fn test_filter_basic() {
    let filter = create_test_filter();
    let result = filter.filter("you are a bastard");

    assert!(result.had_sensitive);
    assert!(!result.text.contains("bastard"));
    assert!(result.text.contains("▓▓▓▓▓▓▓"));
    assert_eq!(result.matches.len(), 1);
  }

  #[test]
  fn test_filter_chinese() {
    let filter = create_test_filter();
    let result = filter.filter("你真是个傻逼");

    assert!(result.had_sensitive);
    assert!(!result.text.contains("傻逼"));
    assert!(result.text.contains("▓▓"));
  }

  #[test]
  fn test_filter_case_insensitive() {
    let filter = create_test_filter();
    let result = filter.filter("YOU ARE A BASTARD");

    assert!(result.had_sensitive);
    assert!(!result.text.to_lowercase().contains("bastard"));
  }

  #[test]
  fn test_filter_high_level() {
    let filter = create_test_filter();
    let result = filter.filter("nigger");

    assert!(result.had_sensitive);
    assert!(result.text.contains("██████"));
    assert_eq!(result.matches[0].level, SensitivityLevel::High);
  }

  #[test]
  fn test_filter_with_audit() {
    let filter = create_test_filter();
    let result = filter.filter_with_audit("you bastard", Some("user123"), Some("room456"));

    assert!(result.had_sensitive);

    // Check stats were updated
    let stats = filter.stats();
    assert_eq!(stats.total_operations, 1);
    assert_eq!(stats.filtered_count, 1);

    // Check events were recorded
    let events = filter.recent_events(10);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].user_id, Some("user123".to_string()));
    assert_eq!(events[0].room_id, Some("room456".to_string()));
  }

  #[test]
  fn test_sensitivity_level_mask_char() {
    assert_eq!(SensitivityLevel::High.mask_char(), '█');
    assert_eq!(SensitivityLevel::Medium.mask_char(), '▓');
    assert_eq!(SensitivityLevel::Low.mask_char(), '░');
  }

  #[test]
  fn test_multiple_matches_different_levels() {
    let filter = create_test_filter();
    let result = filter.filter("damn this bastard");

    assert!(result.had_sensitive);
    assert_eq!(result.matches.len(), 2);

    // Check different mask characters
    assert!(result.text.contains('░')); // damn
    assert!(result.text.contains('▓')); // bastard
  }

  #[test]
  fn test_stats_tracking() {
    let filter = create_test_filter();

    // Multiple operations
    filter.filter_with_audit("damn", None, None);
    filter.filter_with_audit("bastard", None, None);
    filter.filter_with_audit("clean text", None, None);

    let stats = filter.stats();
    assert_eq!(stats.total_operations, 3);
    assert_eq!(stats.filtered_count, 2);

    // Check level breakdown
    assert_eq!(*stats.by_level.get("low").unwrap_or(&0), 1);
    assert_eq!(*stats.by_level.get("medium").unwrap_or(&0), 1);
  }
}
