//! WebSocket signaling client
//!
//! Manages WebSocket connection with the signaling server, including:
//! - Connection establishment and auto-reconnection
//! - Signaling message serialization/deserialization (bitcode)
//! - Heartbeat keepalive
//! - Dispatching received signaling messages to global state

pub(crate) mod handler;

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{BinaryType, CloseEvent, MessageEvent, WebSocket};

use message::signal::SignalMessage;

use crate::state::{self, ConnectionStatus};

/// Maximum reconnection attempts
const MAX_RECONNECT_ATTEMPTS: u32 = 10;
/// Heartbeat interval (milliseconds)
const HEARTBEAT_INTERVAL_MS: i32 = 15_000;
/// Initial reconnection delay (milliseconds)
const INITIAL_RECONNECT_DELAY_MS: u32 = 1_000;

/// WebSocket client manager
///
/// Shared to the component tree via `provide_context`,
/// components can obtain it via `use_context::<WsClient>()` to send signaling messages.
#[derive(Clone)]
pub struct WsClient {
  /// Underlying WebSocket instance
  ws: StoredValue<Option<WebSocket>>,
  /// Server URL
  url: StoredValue<String>,
  /// Reconnect timer ID
  reconnect_timer: StoredValue<Option<i32>>,
  /// Heartbeat timer ID
  heartbeat_timer: StoredValue<Option<i32>>,
}

impl WsClient {
  /// Create a new WebSocket client and provide it to context
  pub fn provide(url: &str) {
    let client = Self {
      ws: StoredValue::new(None),
      url: StoredValue::new(url.to_string()),
      reconnect_timer: StoredValue::new(None),
      heartbeat_timer: StoredValue::new(None),
    };
    provide_context(client);
  }

  /// Get WsClient from context
  pub fn use_client() -> Self {
    use_context::<Self>().expect("WsClient not provided")
  }

  /// Establish WebSocket connection
  pub fn connect(&self) {
    let conn_state = state::use_connection_state();
    conn_state.update(|s| s.ws_status = ConnectionStatus::Connecting);

    let url = self.url.get_value();
    let ws = match WebSocket::new(&url) {
      Ok(ws) => ws,
      Err(e) => {
        web_sys::console::error_1(&format!("WebSocket creation failed: {e:?}").into());
        conn_state.update(|s| s.ws_status = ConnectionStatus::Disconnected);
        return;
      }
    };

    ws.set_binary_type(BinaryType::Arraybuffer);

    // ---- onopen ----
    let conn_state_open = conn_state;
    let self_clone = self.clone();
    let onopen = Closure::<dyn Fn()>::new(move || {
      web_sys::console::log_1(&"WebSocket connected".into());
      conn_state_open.update(|s| {
        s.ws_status = ConnectionStatus::Connected;
        s.reconnect_count = 0;
      });
      self_clone.start_heartbeat();
      let user_state = state::use_user_state();
      let token = user_state.get_untracked().token.clone();
      if !token.is_empty() {
        let _ = self_clone.send(&SignalMessage::TokenAuth { token });
      }
    });
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    // ---- onmessage ----
    let onmessage = Closure::<dyn Fn(MessageEvent)>::new(move |ev: MessageEvent| {
      if let Ok(buf) = ev.data().dyn_into::<js_sys::ArrayBuffer>() {
        let array = js_sys::Uint8Array::new(&buf);
        let bytes = array.to_vec();
        match bitcode::deserialize::<SignalMessage>(&bytes) {
          Ok(msg) => handler::handle_signal_message(msg),
          Err(e) => {
            web_sys::console::warn_1(
              &format!("Signaling message deserialization failed: {e}").into(),
            );
          }
        }
      }
    });
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // ---- onerror ----
    let onerror = Closure::<dyn Fn()>::new(move || {
      web_sys::console::error_1(&"WebSocket error".into());
    });
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();

    // ---- onclose ----
    let self_clone2 = self.clone();
    let conn_state_close = conn_state;
    let onclose = Closure::<dyn Fn(CloseEvent)>::new(move |ev: CloseEvent| {
      web_sys::console::log_1(
        &format!(
          "WebSocket closed: code={}, reason={}",
          ev.code(),
          ev.reason()
        )
        .into(),
      );
      self_clone2.stop_heartbeat();
      conn_state_close.update(|s| s.ws_status = ConnectionStatus::Disconnected);
      self_clone2.schedule_reconnect();
    });
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    self.ws.set_value(Some(ws));
  }

  /// Send signaling message
  pub fn send(&self, msg: &SignalMessage) -> Result<(), String> {
    let ws = self.ws.get_value();
    let ws = ws.as_ref().ok_or("WebSocket not connected")?;

    if ws.ready_state() != WebSocket::OPEN {
      return Err("WebSocket not in OPEN state".to_string());
    }

    let bytes = bitcode::serialize(msg).map_err(|e| format!("Serialization failed: {e}"))?;
    ws.send_with_u8_array(&bytes)
      .map_err(|e| format!("Send failed: {e:?}"))
  }

  /// Disconnect
  pub fn disconnect(&self) {
    self.stop_heartbeat();
    self.clear_reconnect_timer();

    if let Some(ws) = self.ws.get_value() {
      let _ = ws.close();
    }
    self.ws.set_value(None);

    let conn_state = state::use_connection_state();
    conn_state.update(|s| {
      s.ws_status = ConnectionStatus::Disconnected;
      s.reconnect_count = 0;
    });
  }

  /// Start heartbeat timer
  fn start_heartbeat(&self) {
    self.stop_heartbeat();

    let self_clone = self.clone();
    let cb = Closure::<dyn Fn()>::new(move || {
      let _ = self_clone.send(&SignalMessage::Ping);
    });

    if let Some(window) = web_sys::window() {
      let id = window
        .set_interval_with_callback_and_timeout_and_arguments_0(
          cb.as_ref().unchecked_ref(),
          HEARTBEAT_INTERVAL_MS,
        )
        .unwrap_or(0);
      self.heartbeat_timer.set_value(Some(id));
    }
    cb.forget();
  }

  /// Stop heartbeat timer
  fn stop_heartbeat(&self) {
    if let Some(id) = self.heartbeat_timer.get_value() {
      if let Some(window) = web_sys::window() {
        window.clear_interval_with_handle(id);
      }
      self.heartbeat_timer.set_value(None);
    }
  }

  /// Schedule automatic reconnection (exponential backoff)
  fn schedule_reconnect(&self) {
    let conn_state = state::use_connection_state();
    let count = conn_state.get_untracked().reconnect_count;

    if count >= MAX_RECONNECT_ATTEMPTS {
      web_sys::console::warn_1(
        &"Maximum reconnection attempts reached, stopping reconnection".into(),
      );
      return;
    }

    conn_state.update(|s| {
      s.ws_status = ConnectionStatus::Reconnecting;
      s.reconnect_count += 1;
    });

    let delay = (INITIAL_RECONNECT_DELAY_MS * 2u32.pow(count)).min(30_000);
    web_sys::console::log_1(&format!("Reconnecting in {}ms (attempt {})", delay, count + 1).into());

    let self_clone = self.clone();
    let cb = Closure::<dyn Fn()>::new(move || {
      self_clone.connect();
    });

    if let Some(window) = web_sys::window() {
      let id = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
          cb.as_ref().unchecked_ref(),
          delay.cast_signed(),
        )
        .unwrap_or(0);
      self.reconnect_timer.set_value(Some(id));
    }
    cb.forget();
  }

  /// Clear reconnection timer
  fn clear_reconnect_timer(&self) {
    if let Some(id) = self.reconnect_timer.get_value() {
      if let Some(window) = web_sys::window() {
        window.clear_timeout_with_handle(id);
      }
      self.reconnect_timer.set_value(None);
    }
  }
}
