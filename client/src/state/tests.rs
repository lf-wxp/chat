//! State module tests

#[cfg(test)]
mod tests {
  use wasm_bindgen_test::wasm_bindgen_test;

  use crate::state::*;

  // =========================================================================
  // Theme Tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_theme_default() {
    assert_eq!(Theme::default(), Theme::System);
  }

  #[wasm_bindgen_test]
  fn test_theme_equality() {
    assert_eq!(Theme::Light, Theme::Light);
    assert_ne!(Theme::Light, Theme::Dark);
    assert_ne!(Theme::System, Theme::Light);
  }

  #[wasm_bindgen_test]
  fn test_theme_copy() {
    let theme = Theme::Dark;
    let copied = theme;
    assert_eq!(theme, copied);
  }

  // =========================================================================
  // VadState Tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_vad_state_default() {
    let state = VadState::default();
    assert!(state.speaking_users.is_empty());
    assert!(state.volume_levels.is_empty());
  }

  #[wasm_bindgen_test]
  fn test_vad_state_is_speaking() {
    let mut state = VadState::default();
    assert!(!state.is_speaking("user-1"));

    state.speaking_users.insert("user-1".to_string());
    assert!(state.is_speaking("user-1"));
    assert!(!state.is_speaking("user-2"));
  }

  #[wasm_bindgen_test]
  fn test_vad_state_volume() {
    let mut state = VadState::default();
    assert!((state.volume("user-1") - 0.0).abs() < f64::EPSILON);

    state.volume_levels.insert("user-1".to_string(), 75.5);
    assert!((state.volume("user-1") - 75.5).abs() < f64::EPSILON);
    assert!((state.volume("user-2") - 0.0).abs() < f64::EPSILON);
  }

  #[wasm_bindgen_test]
  fn test_vad_state_multiple_users() {
    let mut state = VadState::default();
    state.speaking_users.insert("user-1".to_string());
    state.speaking_users.insert("user-2".to_string());
    state.volume_levels.insert("user-1".to_string(), 80.0);
    state.volume_levels.insert("user-2".to_string(), 30.0);

    assert!(state.is_speaking("user-1"));
    assert!(state.is_speaking("user-2"));
    assert!((state.volume("user-1") - 80.0).abs() < f64::EPSILON);
    assert!((state.volume("user-2") - 30.0).abs() < f64::EPSILON);
  }

  // =========================================================================
  // NetworkQualityState Tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_network_quality_state_default() {
    let state = NetworkQualityState::default();
    assert!(state.peer_stats.is_empty());
  }

  #[wasm_bindgen_test]
  fn test_network_quality_state_quality_unknown_peer() {
    let state = NetworkQualityState::default();
    assert_eq!(
      state.quality("unknown"),
      crate::network_quality::QualityLevel::default()
    );
  }

  #[wasm_bindgen_test]
  fn test_network_quality_state_quality_known_peer() {
    let mut state = NetworkQualityState::default();
    state.peer_stats.insert(
      "user-1".to_string(),
      crate::network_quality::NetworkStats {
        quality: crate::network_quality::QualityLevel::Excellent,
        ..Default::default()
      },
    );
    assert_eq!(
      state.quality("user-1"),
      crate::network_quality::QualityLevel::Excellent
    );
  }

  #[wasm_bindgen_test]
  fn test_network_quality_state_worst_quality_empty() {
    let state = NetworkQualityState::default();
    assert_eq!(
      state.worst_quality(),
      crate::network_quality::QualityLevel::default()
    );
  }

  #[wasm_bindgen_test]
  fn test_network_quality_state_worst_quality_mixed() {
    let mut state = NetworkQualityState::default();
    state.peer_stats.insert(
      "user-1".to_string(),
      crate::network_quality::NetworkStats {
        quality: crate::network_quality::QualityLevel::Excellent,
        ..Default::default()
      },
    );
    state.peer_stats.insert(
      "user-2".to_string(),
      crate::network_quality::NetworkStats {
        quality: crate::network_quality::QualityLevel::Poor,
        ..Default::default()
      },
    );
    assert_eq!(
      state.worst_quality(),
      crate::network_quality::QualityLevel::Poor
    );
  }

  #[wasm_bindgen_test]
  fn test_network_quality_state_worst_quality_all_excellent() {
    let mut state = NetworkQualityState::default();
    for i in 0..3 {
      state.peer_stats.insert(
        format!("user-{i}"),
        crate::network_quality::NetworkStats {
          quality: crate::network_quality::QualityLevel::Excellent,
          ..Default::default()
        },
      );
    }
    assert_eq!(
      state.worst_quality(),
      crate::network_quality::QualityLevel::Excellent
    );
  }

  // =========================================================================
  // ConnectionStatus Tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_connection_status_default() {
    assert_eq!(ConnectionStatus::default(), ConnectionStatus::Disconnected);
  }

  #[wasm_bindgen_test]
  fn test_connection_status_equality() {
    assert_eq!(ConnectionStatus::Connected, ConnectionStatus::Connected);
    assert_ne!(ConnectionStatus::Connected, ConnectionStatus::Disconnected);
  }

  // =========================================================================
  // UserState Tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_user_state_default() {
    let state = UserState::default();
    assert!(!state.authenticated);
    assert!(state.user_id.is_empty());
    assert!(state.username.is_empty());
    assert!(state.token.is_empty());
  }

  // =========================================================================
  // ChatState Tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_chat_state_default() {
    let state = ChatState::default();
    assert!(state.conversations.is_empty());
    assert!(state.active_conversation_id.is_none());
    assert!(state.messages.is_empty());
  }

  // =========================================================================
  // ToastType Tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_toast_type_equality() {
    assert_eq!(ToastType::Success, ToastType::Success);
    assert_ne!(ToastType::Success, ToastType::Error);
    assert_ne!(ToastType::Warning, ToastType::Info);
  }

  // =========================================================================
  // ModalType Tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_modal_type_equality() {
    assert_eq!(ModalType::CreateRoom, ModalType::CreateRoom);
    assert_ne!(ModalType::CreateRoom, ModalType::CreateTheater);
  }

  #[wasm_bindgen_test]
  fn test_modal_type_user_profile() {
    let modal = ModalType::UserProfile("user-1".to_string());
    assert_eq!(modal, ModalType::UserProfile("user-1".to_string()));
    assert_ne!(modal, ModalType::UserProfile("user-2".to_string()));
  }

  // =========================================================================
  // NetworkQualityState Alert Method Tests
  // =========================================================================

  fn make_alert(
    id: &str,
    severity: crate::network_quality::AlertSeverity,
    acknowledged: bool,
  ) -> crate::network_quality::NetworkAlert {
    crate::network_quality::NetworkAlert {
      id: id.to_string(),
      peer_id: "peer-1".to_string(),
      alert_type: crate::network_quality::AlertType::HighRtt,
      severity,
      message: format!("Alert {id}"),
      metrics: crate::network_quality::NetworkStats::default(),
      timestamp: 1000.0,
      acknowledged,
    }
  }

  #[wasm_bindgen_test]
  fn test_acknowledge_alert() {
    let mut state = NetworkQualityState::default();
    state.alerts.push(make_alert(
      "a1",
      crate::network_quality::AlertSeverity::Warning,
      false,
    ));
    state.alerts.push(make_alert(
      "a2",
      crate::network_quality::AlertSeverity::Critical,
      false,
    ));
    state.unacknowledged_alert_count = 2;

    state.acknowledge_alert("a1");
    assert!(state.alerts[0].acknowledged);
    assert!(!state.alerts[1].acknowledged);
    assert_eq!(state.unacknowledged_alert_count, 1);

    // Acknowledging the same alert again should be a no-op
    state.acknowledge_alert("a1");
    assert_eq!(state.unacknowledged_alert_count, 1);

    // Acknowledging a non-existent alert should be a no-op
    state.acknowledge_alert("non-existent");
    assert_eq!(state.unacknowledged_alert_count, 1);
  }

  #[wasm_bindgen_test]
  fn test_acknowledge_all_alerts() {
    let mut state = NetworkQualityState::default();
    state.alerts.push(make_alert(
      "a1",
      crate::network_quality::AlertSeverity::Warning,
      false,
    ));
    state.alerts.push(make_alert(
      "a2",
      crate::network_quality::AlertSeverity::Critical,
      false,
    ));
    state.alerts.push(make_alert(
      "a3",
      crate::network_quality::AlertSeverity::Warning,
      true,
    ));
    state.unacknowledged_alert_count = 2;

    state.acknowledge_all_alerts();
    assert!(state.alerts.iter().all(|a| a.acknowledged));
    assert_eq!(state.unacknowledged_alert_count, 0);
  }

  #[wasm_bindgen_test]
  fn test_clear_acknowledged_alerts() {
    let mut state = NetworkQualityState::default();
    state.alerts.push(make_alert(
      "a1",
      crate::network_quality::AlertSeverity::Warning,
      true,
    ));
    state.alerts.push(make_alert(
      "a2",
      crate::network_quality::AlertSeverity::Critical,
      false,
    ));
    state.alerts.push(make_alert(
      "a3",
      crate::network_quality::AlertSeverity::Warning,
      true,
    ));

    state.clear_acknowledged_alerts();
    assert_eq!(state.alerts.len(), 1);
    assert_eq!(state.alerts[0].id, "a2");
  }

  #[wasm_bindgen_test]
  fn test_critical_alerts() {
    let mut state = NetworkQualityState::default();
    state.alerts.push(make_alert(
      "a1",
      crate::network_quality::AlertSeverity::Warning,
      false,
    ));
    state.alerts.push(make_alert(
      "a2",
      crate::network_quality::AlertSeverity::Critical,
      false,
    ));
    state.alerts.push(make_alert(
      "a3",
      crate::network_quality::AlertSeverity::Critical,
      true,
    ));

    let critical = state.critical_alerts();
    assert_eq!(critical.len(), 2);
    assert!(
      critical
        .iter()
        .all(|a| a.severity == crate::network_quality::AlertSeverity::Critical)
    );
  }

  #[wasm_bindgen_test]
  fn test_warning_alerts() {
    let mut state = NetworkQualityState::default();
    state.alerts.push(make_alert(
      "a1",
      crate::network_quality::AlertSeverity::Warning,
      false,
    ));
    state.alerts.push(make_alert(
      "a2",
      crate::network_quality::AlertSeverity::Critical,
      false,
    ));
    state.alerts.push(make_alert(
      "a3",
      crate::network_quality::AlertSeverity::Warning,
      false,
    ));

    let warnings = state.warning_alerts();
    assert_eq!(warnings.len(), 2);
    assert!(
      warnings
        .iter()
        .all(|a| a.severity == crate::network_quality::AlertSeverity::Warning)
    );
  }

  // =========================================================================
  // SearchState Tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_search_state_default() {
    let state = SearchState::default();
    assert!(state.query.is_empty());
    assert!(!state.is_searching);
    assert!(state.results.is_empty());
    assert!(!state.show_panel);
    assert!(state.in_chat_matches.is_empty());
    assert_eq!(state.in_chat_current_index, 0);
  }
}
