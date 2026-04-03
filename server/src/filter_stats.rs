//! Sensitive word filtering statistics and audit logging module.
//!
//! Tracks filtering events, provides statistics, and maintains audit logs
//! for compliance and monitoring purposes.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

// =============================================================================
// Filter Event Types
// =============================================================================

/// Context where sensitive word filtering occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilterContext {
  /// Room name
  RoomName,
  /// Room description
  RoomDescription,
  /// Chat message
  ChatMessage,
  /// Danmaku (bullet comment)
  Danmaku,
  /// User signature
  UserSignature,
  /// Invite message
  InviteMessage,
}

impl FilterContext {
  /// Convert to string for logging
  pub fn as_str(self) -> &'static str {
    match self {
      Self::RoomName => "room_name",
      Self::RoomDescription => "room_description",
      Self::ChatMessage => "chat_message",
      Self::Danmaku => "danmaku",
      Self::UserSignature => "user_signature",
      Self::InviteMessage => "invite_message",
    }
  }
}

/// Severity level of filtered content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
  /// High severity (e.g., hate speech, severe profanity)
  High,
  /// Medium severity (e.g., moderate profanity)
  Medium,
  /// Low severity (e.g., mild inappropriate content)
  Low,
}

impl Severity {
  /// Convert to string for logging
  pub fn as_str(self) -> &'static str {
    match self {
      Self::High => "high",
      Self::Medium => "medium",
      Self::Low => "low",
    }
  }
}

// =============================================================================
// Filter Event Record
// =============================================================================

/// A single filter event record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterEvent {
  /// Unique event ID
  pub id: u64,
  /// Timestamp when the event occurred
  pub timestamp: DateTime<Utc>,
  /// User ID who triggered the filter
  pub user_id: String,
  /// Room ID (if applicable)
  pub room_id: Option<String>,
  /// Context where filtering occurred
  pub context: FilterContext,
  /// Words that were filtered
  pub filtered_words: Vec<String>,
  /// Severity level
  pub severity: Severity,
  /// Original text (truncated for privacy, max 100 chars)
  pub original_text_preview: String,
  /// Whether the event was logged for audit
  pub audit_logged: bool,
}

// =============================================================================
// Statistics Aggregates
// =============================================================================

/// Statistics for a single word.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WordStats {
  /// Total occurrences
  pub count: u64,
  /// Occurrences by context
  pub by_context: HashMap<String, u64>,
  /// Occurrences by severity
  pub by_severity: HashMap<String, u64>,
  /// First occurrence timestamp
  pub first_seen: Option<DateTime<Utc>>,
  /// Last occurrence timestamp
  pub last_seen: Option<DateTime<Utc>>,
}

/// Global filter statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilterStatistics {
  /// Total filter events
  pub total_events: u64,
  /// Events by context
  pub events_by_context: HashMap<String, u64>,
  /// Events by severity
  pub events_by_severity: HashMap<String, u64>,
  /// Events by user (top users)
  pub events_by_user: HashMap<String, u64>,
  /// Per-word statistics
  pub word_stats: HashMap<String, WordStats>,
  /// Events in the last hour
  pub events_last_hour: u64,
  /// Events in the last 24 hours
  pub events_last_24h: u64,
}

// =============================================================================
// Filter Statistics Manager
// =============================================================================

/// Thread-safe manager for filter statistics and audit logging.
#[derive(Clone)]
pub struct FilterStatsManager {
  /// Next event ID (atomic counter)
  next_event_id: Arc<AtomicU64>,
  /// Recent filter events (in-memory, circular buffer style)
  recent_events: Arc<DashMap<u64, FilterEvent>>,
  /// Maximum number of recent events to keep in memory
  max_recent_events: usize,
  /// Per-word statistics
  word_stats: Arc<DashMap<String, WordStats>>,
  /// Events by context (atomic counters)
  events_by_context: Arc<DashMap<String, AtomicU64>>,
  /// Events by severity (atomic counters)
  events_by_severity: Arc<DashMap<String, AtomicU64>>,
  /// Events by user
  events_by_user: Arc<DashMap<String, AtomicU64>>,
  /// Total events counter
  total_events: Arc<AtomicU64>,
}

impl FilterStatsManager {
  /// Create a new statistics manager.
  pub fn new(max_recent_events: usize) -> Self {
    Self {
      next_event_id: Arc::new(AtomicU64::new(1)),
      recent_events: Arc::new(DashMap::new()),
      max_recent_events,
      word_stats: Arc::new(DashMap::new()),
      events_by_context: Arc::new(DashMap::new()),
      events_by_severity: Arc::new(DashMap::new()),
      events_by_user: Arc::new(DashMap::new()),
      total_events: Arc::new(AtomicU64::new(0)),
    }
  }

  /// Record a filter event.
  ///
  /// This method:
  /// 1. Creates a filter event record
  /// 2. Updates statistics
  /// 3. Logs to tracing
  /// 4. Returns the event ID
  pub fn record_event(
    &self,
    user_id: String,
    room_id: Option<String>,
    context: FilterContext,
    filtered_words: Vec<String>,
    severity: Severity,
    original_text: &str,
  ) -> u64 {
    // Generate event ID
    let event_id = self.next_event_id.fetch_add(1, Ordering::Relaxed);

    // Truncate original text for privacy
    let original_text_preview = if original_text.chars().count() > 100 {
      let chars: Vec<char> = original_text.chars().take(97).collect();
      format!("{}...", chars.into_iter().collect::<String>())
    } else {
      original_text.to_string()
    };

    // Create event record
    let event = FilterEvent {
      id: event_id,
      timestamp: Utc::now(),
      user_id: user_id.clone(),
      room_id: room_id.clone(),
      context,
      filtered_words: filtered_words.clone(),
      severity,
      original_text_preview,
      audit_logged: false,
    };

    // Update statistics
    self.update_stats(&event, &filtered_words);

    // Log to tracing
    self.log_event(&event);

    // Store in recent events (with cleanup)
    self.store_event(event);

    event_id
  }

  /// Update statistics for an event.
  fn update_stats(&self, event: &FilterEvent, filtered_words: &[String]) {
    // Increment total events
    self.total_events.fetch_add(1, Ordering::Relaxed);

    // Update context counter
    self
      .events_by_context
      .entry(event.context.as_str().to_string())
      .or_insert_with(|| AtomicU64::new(0))
      .fetch_add(1, Ordering::Relaxed);

    // Update severity counter
    self
      .events_by_severity
      .entry(event.severity.as_str().to_string())
      .or_insert_with(|| AtomicU64::new(0))
      .fetch_add(1, Ordering::Relaxed);

    // Update user counter
    self
      .events_by_user
      .entry(event.user_id.clone())
      .or_insert_with(|| AtomicU64::new(0))
      .fetch_add(1, Ordering::Relaxed);

    // Update per-word statistics
    for word in filtered_words {
      let mut stats = self
        .word_stats
        .entry(word.clone())
        .or_insert_with(WordStats::default);

      stats.count += 1;
      stats.last_seen = Some(event.timestamp);

      if stats.first_seen.is_none() {
        stats.first_seen = Some(event.timestamp);
      }

      // Update context breakdown
      *stats
        .by_context
        .entry(event.context.as_str().to_string())
        .or_insert(0) += 1;

      // Update severity breakdown
      *stats
        .by_severity
        .entry(event.severity.as_str().to_string())
        .or_insert(0) += 1;
    }
  }

  /// Log event to tracing.
  fn log_event(&self, event: &FilterEvent) {
    let words_joined = event.filtered_words.join(", ");
    let severity_str = event.severity.as_str();
    let context_str = event.context.as_str();

    match event.severity {
      Severity::High => {
        error!(
          target: "sensitive_filter",
          event_id = event.id,
          user_id = %event.user_id,
          room_id = ?event.room_id,
          context = context_str,
          severity = severity_str,
          words = %words_joined,
          "High severity sensitive content filtered"
        );
      }
      Severity::Medium => {
        warn!(
          target: "sensitive_filter",
          event_id = event.id,
          user_id = %event.user_id,
          room_id = ?event.room_id,
          context = context_str,
          severity = severity_str,
          words = %words_joined,
          "Medium severity sensitive content filtered"
        );
      }
      Severity::Low => {
        info!(
          target: "sensitive_filter",
          event_id = event.id,
          user_id = %event.user_id,
          room_id = ?event.room_id,
          context = context_str,
          severity = severity_str,
          words = %words_joined,
          "Low severity sensitive content filtered"
        );
      }
    }
  }

  /// Store event in recent events buffer.
  fn store_event(&self, event: FilterEvent) {
    // Simple cleanup: remove old events if we exceed the limit
    if self.recent_events.len() >= self.max_recent_events {
      // Find and remove the oldest event
      let mut oldest_id: Option<u64> = None;
      for entry in self.recent_events.iter() {
        if oldest_id.is_none() || entry.key() < oldest_id.as_ref().unwrap() {
          oldest_id = Some(*entry.key());
        }
      }
      if let Some(id) = oldest_id {
        self.recent_events.remove(&id);
      }
    }

    self.recent_events.insert(event.id, event);
  }

  /// Get statistics snapshot.
  #[must_use]
  pub fn get_statistics(&self) -> FilterStatistics {
    let mut stats = FilterStatistics::default();

    stats.total_events = self.total_events.load(Ordering::Relaxed);

    // Collect context stats
    for entry in self.events_by_context.iter() {
      stats
        .events_by_context
        .insert(entry.key().clone(), entry.load(Ordering::Relaxed));
    }

    // Collect severity stats
    for entry in self.events_by_severity.iter() {
      stats
        .events_by_severity
        .insert(entry.key().clone(), entry.load(Ordering::Relaxed));
    }

    // Collect user stats
    for entry in self.events_by_user.iter() {
      stats
        .events_by_user
        .insert(entry.key().clone(), entry.load(Ordering::Relaxed));
    }

    // Collect word stats
    for entry in self.word_stats.iter() {
      stats
        .word_stats
        .insert(entry.key().clone(), entry.value().clone());
    }

    // Calculate recent events (events in last hour and 24h)
    let now = Utc::now();
    let one_hour_ago = now - chrono::Duration::hours(1);
    let twenty_four_hours_ago = now - chrono::Duration::hours(24);

    for entry in self.recent_events.iter() {
      if entry.timestamp > one_hour_ago {
        stats.events_last_hour += 1;
      }
      if entry.timestamp > twenty_four_hours_ago {
        stats.events_last_24h += 1;
      }
    }

    stats
  }

  /// Get recent events (optionally filtered by user or room).
  #[must_use]
  pub fn get_recent_events(
    &self,
    user_id: Option<&str>,
    room_id: Option<&str>,
    limit: usize,
  ) -> Vec<FilterEvent> {
    let mut events: Vec<FilterEvent> = self
      .recent_events
      .iter()
      .filter(|entry| {
        if let Some(uid) = user_id {
          if entry.user_id != uid {
            return false;
          }
        }
        if let Some(rid) = room_id {
          if entry.room_id.as_deref() != Some(rid) {
            return false;
          }
        }
        true
      })
      .map(|entry| entry.value().clone())
      .collect();

    // Sort by timestamp (most recent first)
    events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    events.truncate(limit);
    events
  }

  /// Get statistics for a specific word.
  #[must_use]
  pub fn get_word_stats(&self, word: &str) -> Option<WordStats> {
    self.word_stats.get(word).map(|entry| entry.value().clone())
  }

  /// Get top N words by occurrence count.
  #[must_use]
  pub fn get_top_words(&self, limit: usize) -> Vec<(String, WordStats)> {
    let mut words: Vec<(String, WordStats)> = self
      .word_stats
      .iter()
      .map(|entry| (entry.key().clone(), entry.value().clone()))
      .collect();

    words.sort_by(|a, b| b.1.count.cmp(&a.1.count));
    words.truncate(limit);
    words
  }

  /// Get top N users by filter event count.
  #[must_use]
  pub fn get_top_users(&self, limit: usize) -> Vec<(String, u64)> {
    let mut users: Vec<(String, u64)> = self
      .events_by_user
      .iter()
      .map(|entry| (entry.key().clone(), entry.load(Ordering::Relaxed)))
      .collect();

    users.sort_by(|a, b| b.1.cmp(&a.1));
    users.truncate(limit);
    users
  }

  /// Clear all statistics (useful for testing or reset).
  pub fn clear(&self) {
    self.next_event_id.store(1, Ordering::Relaxed);
    self.recent_events.clear();
    self.word_stats.clear();
    self.events_by_context.clear();
    self.events_by_severity.clear();
    self.events_by_user.clear();
    self.total_events.store(0, Ordering::Relaxed);
  }
}

impl Default for FilterStatsManager {
  fn default() -> Self {
    Self::new(1000) // Default: keep last 1000 events in memory
  }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_filter_context_as_str() {
    assert_eq!(FilterContext::RoomName.as_str(), "room_name");
    assert_eq!(FilterContext::ChatMessage.as_str(), "chat_message");
    assert_eq!(FilterContext::Danmaku.as_str(), "danmaku");
  }

  #[test]
  fn test_severity_as_str() {
    assert_eq!(Severity::High.as_str(), "high");
    assert_eq!(Severity::Medium.as_str(), "medium");
    assert_eq!(Severity::Low.as_str(), "low");
  }

  #[test]
  fn test_stats_manager_record_event() {
    let manager = FilterStatsManager::new(100);

    let event_id = manager.record_event(
      "user123".to_string(),
      Some("room456".to_string()),
      FilterContext::ChatMessage,
      vec!["word1".to_string(), "word2".to_string()],
      Severity::Medium,
      "This is a test message with bad words",
    );

    assert!(event_id > 0);

    let stats = manager.get_statistics();
    assert_eq!(stats.total_events, 1);
    assert_eq!(stats.events_by_context.get("chat_message"), Some(&1));
    assert_eq!(stats.events_by_severity.get("medium"), Some(&1));
    assert_eq!(stats.events_by_user.get("user123"), Some(&1));
  }

  #[test]
  fn test_stats_manager_multiple_events() {
    let manager = FilterStatsManager::new(100);

    manager.record_event(
      "user1".to_string(),
      None,
      FilterContext::RoomName,
      vec!["badword".to_string()],
      Severity::High,
      "Bad room name",
    );

    manager.record_event(
      "user2".to_string(),
      Some("room1".to_string()),
      FilterContext::ChatMessage,
      vec!["badword".to_string()],
      Severity::Medium,
      "Bad message",
    );

    manager.record_event(
      "user1".to_string(),
      Some("room1".to_string()),
      FilterContext::Danmaku,
      vec!["badword".to_string(), "another".to_string()],
      Severity::Low,
      "Bad danmaku",
    );

    let stats = manager.get_statistics();
    assert_eq!(stats.total_events, 3);
    assert_eq!(stats.events_by_user.get("user1"), Some(&2));
    assert_eq!(stats.events_by_user.get("user2"), Some(&1));

    // Word "badword" should appear in all 3 events
    let word_stats = manager.get_word_stats("badword").unwrap();
    assert_eq!(word_stats.count, 3);
  }

  #[test]
  fn test_stats_manager_get_top_words() {
    let manager = FilterStatsManager::new(100);

    // Create events with different word frequencies
    for i in 0..5 {
      manager.record_event(
        format!("user{i}"),
        None,
        FilterContext::ChatMessage,
        vec!["common".to_string()],
        Severity::Medium,
        "test",
      );
    }

    for i in 0..3 {
      manager.record_event(
        format!("user{i}"),
        None,
        FilterContext::ChatMessage,
        vec!["less_common".to_string()],
        Severity::Medium,
        "test",
      );
    }

    manager.record_event(
      "user0".to_string(),
      None,
      FilterContext::ChatMessage,
      vec!["rare".to_string()],
      Severity::Medium,
      "test",
    );

    let top_words = manager.get_top_words(10);
    assert_eq!(top_words.len(), 3);
    assert_eq!(top_words[0].0, "common");
    assert_eq!(top_words[0].1.count, 5);
    assert_eq!(top_words[1].0, "less_common");
    assert_eq!(top_words[1].1.count, 3);
    assert_eq!(top_words[2].0, "rare");
    assert_eq!(top_words[2].1.count, 1);
  }

  #[test]
  fn test_stats_manager_get_recent_events() {
    let manager = FilterStatsManager::new(100);

    manager.record_event(
      "user1".to_string(),
      Some("room1".to_string()),
      FilterContext::ChatMessage,
      vec!["word".to_string()],
      Severity::Medium,
      "test 1",
    );

    manager.record_event(
      "user2".to_string(),
      Some("room1".to_string()),
      FilterContext::ChatMessage,
      vec!["word".to_string()],
      Severity::Medium,
      "test 2",
    );

    manager.record_event(
      "user1".to_string(),
      Some("room2".to_string()),
      FilterContext::ChatMessage,
      vec!["word".to_string()],
      Severity::Medium,
      "test 3",
    );

    // Filter by user
    let user1_events = manager.get_recent_events(Some("user1"), None, 10);
    assert_eq!(user1_events.len(), 2);

    // Filter by room
    let room1_events = manager.get_recent_events(None, Some("room1"), 10);
    assert_eq!(room1_events.len(), 2);

    // Filter by both
    let filtered = manager.get_recent_events(Some("user1"), Some("room2"), 10);
    assert_eq!(filtered.len(), 1);
  }

  #[test]
  fn test_stats_manager_max_recent_events() {
    let manager = FilterStatsManager::new(10);

    // Create 15 events
    for i in 0..15 {
      manager.record_event(
        format!("user{i}"),
        None,
        FilterContext::ChatMessage,
        vec!["word".to_string()],
        Severity::Medium,
        "test",
      );
    }

    // Should only keep the last 10
    let stats = manager.get_statistics();
    assert_eq!(stats.total_events, 15); // Total counter keeps counting

    // But recent events buffer should be limited
    let recent = manager.get_recent_events(None, None, 100);
    assert!(recent.len() <= 10);
  }

  #[test]
  fn test_stats_manager_clear() {
    let manager = FilterStatsManager::new(100);

    manager.record_event(
      "user1".to_string(),
      None,
      FilterContext::ChatMessage,
      vec!["word".to_string()],
      Severity::Medium,
      "test",
    );

    assert!(manager.get_statistics().total_events > 0);

    manager.clear();

    let stats = manager.get_statistics();
    assert_eq!(stats.total_events, 0);
    assert!(stats.word_stats.is_empty());
  }

  #[test]
  fn test_text_preview_truncation() {
    let manager = FilterStatsManager::new(100);

    let long_text = "a".repeat(150);
    manager.record_event(
      "user1".to_string(),
      None,
      FilterContext::ChatMessage,
      vec!["word".to_string()],
      Severity::Medium,
      &long_text,
    );

    let events = manager.get_recent_events(None, None, 1);
    assert_eq!(events.len(), 1);
    assert!(events[0].original_text_preview.len() <= 103); // 100 chars + "..."
    assert!(events[0].original_text_preview.ends_with("..."));
  }
}
