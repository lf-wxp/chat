//! Theme preference state

/// Theme preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
  /// Follow system
  #[default]
  System,
  /// Light mode
  Light,
  /// Dark mode
  Dark,
}

/// Theme state
#[derive(Debug, Clone, Default)]
pub struct ThemeState {
  pub theme: Theme,
}
