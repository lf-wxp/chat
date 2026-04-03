//! WebSocket connection state

/// WebSocket connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionStatus {
  /// Not connected
  #[default]
  Disconnected,
  /// Connecting
  Connecting,
  /// Connected
  Connected,
  /// Reconnecting
  Reconnecting,
}

/// Network connection state
#[derive(Debug, Clone, Default)]
pub struct ConnectionState {
  /// WebSocket connection status
  pub ws_status: ConnectionStatus,
  /// Reconnection count
  pub reconnect_count: u32,
}
