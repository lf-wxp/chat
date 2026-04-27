use super::*;

#[test]
fn degradation_downgrades_immediately() {
  let mut ctrl = QualityController::new();
  let action = ctrl.observe(NetworkQuality::Fair);
  assert_eq!(action, QualityAction::Apply(VideoProfile::LOW));
  assert_eq!(ctrl.current_quality(), NetworkQuality::Fair);
}

#[test]
fn poor_applies_very_low_profile() {
  let mut ctrl = QualityController::new();
  let action = ctrl.observe(NetworkQuality::Poor);
  assert_eq!(action, QualityAction::Apply(VideoProfile::VERY_LOW));
  assert_eq!(ctrl.current_quality(), NetworkQuality::Poor);
}

#[test]
fn single_good_sample_after_poor_does_not_recover() {
  let mut ctrl = QualityController::with_initial(NetworkQuality::Poor);
  let action = ctrl.observe(NetworkQuality::Good);
  assert_eq!(action, QualityAction::Hold);
  // Still on very-low because recovery has not been sustained.
  assert_eq!(ctrl.applied_profile(), VideoProfile::VERY_LOW);
}

#[test]
fn sustained_recovery_steps_up_quality() {
  let mut ctrl = QualityController::with_initial(NetworkQuality::Poor);

  // First good sample — still holding.
  assert_eq!(ctrl.observe(NetworkQuality::Good), QualityAction::Hold,);
  // Second consecutive good sample — step up to Fair.
  assert_eq!(
    ctrl.observe(NetworkQuality::Good),
    QualityAction::Apply(VideoProfile::LOW),
  );
  assert_eq!(ctrl.current_quality(), NetworkQuality::Fair);
}

#[test]
fn equal_quality_sample_resets_recovery_streak() {
  let mut ctrl = QualityController::with_initial(NetworkQuality::Fair);

  // First good sample — streak = 1.
  assert_eq!(ctrl.observe(NetworkQuality::Good), QualityAction::Hold);
  // Fair sample (equal to current) — resets the streak.
  assert_eq!(ctrl.observe(NetworkQuality::Fair), QualityAction::Hold);
  // Next good sample is again streak = 1, NOT enough to recover.
  assert_eq!(ctrl.observe(NetworkQuality::Good), QualityAction::Hold);
  assert_eq!(ctrl.current_quality(), NetworkQuality::Fair);
}

#[test]
fn dip_during_recovery_downgrades_again() {
  let mut ctrl = QualityController::with_initial(NetworkQuality::Fair);

  let _ = ctrl.observe(NetworkQuality::Good);
  let action = ctrl.observe(NetworkQuality::Poor);
  assert_eq!(action, QualityAction::Apply(VideoProfile::VERY_LOW));
  assert_eq!(ctrl.current_quality(), NetworkQuality::Poor);
}

#[test]
fn idempotent_samples_keep_profile_stable() {
  let mut ctrl = QualityController::new();
  for _ in 0..10 {
    assert_eq!(ctrl.observe(NetworkQuality::Excellent), QualityAction::Hold,);
  }
  assert_eq!(ctrl.applied_profile(), VideoProfile::HIGH);
}
