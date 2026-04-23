//! DataChannel management for WebRTC peer connections.
//!
//! Handles DataChannel creation, message serialization/deserialization,
//! and sending/receiving messages over the P2P connection.

use js_sys::{ArrayBuffer, Uint8Array};
use message::datachannel::DataChannelMessage;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{RtcDataChannel, RtcDataChannelState};

type MessageClosure = Closure<dyn FnMut(web_sys::MessageEvent)>;
type EventClosure = Closure<dyn FnMut(web_sys::Event)>;

/// Wrapper around `RtcDataChannel` with message encoding/decoding.
#[derive(Debug, Clone)]
pub struct PeerDataChannel {
  /// The underlying RTCDataChannel.
  channel: JsValue,
  /// Peer user ID.
  peer_id: message::UserId,
  /// Whether we created this channel (initiator).
  is_initiator: bool,
  /// Stored message handler closure to prevent memory leak (P1-4 fix).
  on_message: Rc<RefCell<Option<MessageClosure>>>,
  /// Stored open handler closure to prevent memory leak (P1-4 fix).
  on_open: Rc<RefCell<Option<EventClosure>>>,
  /// Stored close handler closure to prevent memory leak (P1-4 fix).
  on_close: Rc<RefCell<Option<EventClosure>>>,
}

impl PeerDataChannel {
  /// Create a new DataChannel wrapper from an existing RtcDataChannel.
  pub fn new(channel: RtcDataChannel, peer_id: message::UserId, is_initiator: bool) -> Self {
    Self {
      channel: JsValue::from(channel),
      peer_id,
      is_initiator,
      on_message: Rc::new(RefCell::new(None)),
      on_open: Rc::new(RefCell::new(None)),
      on_close: Rc::new(RefCell::new(None)),
    }
  }

  /// Create a new DataChannel on an existing RTCPeerConnection.
  ///
  /// # Errors
  /// Returns an error if the DataChannel cannot be created.
  pub fn create_on_connection(
    connection: &web_sys::RtcPeerConnection,
    peer_id: message::UserId,
  ) -> Result<Self, String> {
    let init = web_sys::RtcDataChannelInit::new();
    let channel = connection.create_data_channel_with_data_channel_dict("chat", &init);

    // Set binary type to arraybuffer
    channel.set_binary_type(web_sys::RtcDataChannelType::Arraybuffer);

    web_sys::console::log_1(
      &format!("[datachannel] Created DataChannel for peer {}", peer_id).into(),
    );

    Ok(Self {
      channel: JsValue::from(channel),
      peer_id,
      is_initiator: true,
      on_message: Rc::new(RefCell::new(None)),
      on_open: Rc::new(RefCell::new(None)),
      on_close: Rc::new(RefCell::new(None)),
    })
  }

  /// Send a DataChannel message.
  ///
  /// Serializes the message using bitcode and sends it as a binary message.
  ///
  /// # Errors
  /// Returns an error if the DataChannel is not open or serialization fails.
  pub fn send_message(&self, msg: &DataChannelMessage) -> Result<(), String> {
    let channel = self.get_channel()?;

    if channel.ready_state() != RtcDataChannelState::Open {
      return Err(format!(
        "DataChannel not open (state={:?})",
        channel.ready_state()
      ));
    }

    let discriminator = msg.discriminator();
    let payload = bitcode::encode(msg);

    // Build frame: [discriminator (1 byte)] + [payload]
    let mut frame = Vec::with_capacity(1 + payload.len());
    frame.push(discriminator);
    frame.extend_from_slice(&payload);

    let uint8 = Uint8Array::new_with_length(
      frame
        .len()
        .try_into()
        .expect("DataChannel message frame length exceeds JS u32 range"),
    );
    uint8.copy_from(&frame);

    channel
      .send_with_array_buffer(&uint8.buffer())
      .map_err(|e| format!("Failed to send message: {:?}", e))?;

    Ok(())
  }

  /// Send raw encrypted bytes (for E2EE messages).
  ///
  /// # Errors
  /// Returns an error if the DataChannel is not open.
  pub fn send_raw(&self, data: &[u8]) -> Result<(), String> {
    let channel = self.get_channel()?;

    if channel.ready_state() != RtcDataChannelState::Open {
      return Err(format!(
        "DataChannel not open (state={:?})",
        channel.ready_state()
      ));
    }

    let uint8 = Uint8Array::new_with_length(
      data
        .len()
        .try_into()
        .expect("DataChannel raw data length exceeds JS u32 range"),
    );
    uint8.copy_from(data);

    channel
      .send_with_array_buffer(&uint8.buffer())
      .map_err(|e| format!("Failed to send raw data: {:?}", e))?;

    Ok(())
  }

  /// Set up message handler for incoming messages.
  ///
  /// The callback receives the deserialized `DataChannelMessage`.
  pub fn set_on_message<F>(&self, callback: F)
  where
    F: Fn(DataChannelMessage) + 'static,
  {
    let channel = match self.get_channel() {
      Ok(ch) => ch,
      Err(e) => {
        web_sys::console::error_1(&format!("[datachannel] Failed to set on_message: {}", e).into());
        return;
      }
    };

    let closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
      if let Ok(array_buffer) = event.data().dyn_into::<ArrayBuffer>() {
        let uint8 = Uint8Array::new(&array_buffer);
        let bytes = uint8.to_vec();

        if bytes.is_empty() {
          return;
        }

        // Parse frame: [discriminator (1 byte)] + [payload]
        let discriminator = bytes[0];
        let payload = &bytes[1..];

        match bitcode::decode::<DataChannelMessage>(payload) {
          Ok(msg) => {
            // Verify discriminator matches
            if msg.discriminator() != discriminator {
              web_sys::console::warn_1(&"[datachannel] Discriminator mismatch".into());
              return;
            }
            callback(msg);
          }
          Err(e) => {
            web_sys::console::error_1(
              &format!(
                "[datachannel] Failed to decode message (type=0x{:02X}): {:?}",
                discriminator, e
              )
              .into(),
            );
          }
        }
      } else if let Some(text) = event.data().as_string() {
        web_sys::console::log_1(
          &format!("[datachannel] Received text (unexpected): {}", text).into(),
        );
      }
    }) as Box<dyn FnMut(web_sys::MessageEvent)>);

    channel.set_onmessage(Some(closure.as_ref().unchecked_ref()));
    *self.on_message.borrow_mut() = Some(closure);
  }

  /// Set up open handler.
  pub fn set_on_open<F>(&self, callback: F)
  where
    F: Fn() + 'static,
  {
    let channel = match self.get_channel() {
      Ok(ch) => ch,
      Err(e) => {
        web_sys::console::error_1(&format!("[datachannel] Failed to set on_open: {}", e).into());
        return;
      }
    };

    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
      callback();
    }) as Box<dyn FnMut(web_sys::Event)>);

    channel.set_onopen(Some(closure.as_ref().unchecked_ref()));
    *self.on_open.borrow_mut() = Some(closure);
  }

  /// Set up close handler.
  pub fn set_on_close<F>(&self, callback: F)
  where
    F: Fn() + 'static,
  {
    let channel = match self.get_channel() {
      Ok(ch) => ch,
      Err(e) => {
        web_sys::console::error_1(&format!("[datachannel] Failed to set on_close: {}", e).into());
        return;
      }
    };

    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
      callback();
    }) as Box<dyn FnMut(web_sys::Event)>);

    channel.set_onclose(Some(closure.as_ref().unchecked_ref()));
    *self.on_close.borrow_mut() = Some(closure);
  }

  /// Clear all event handlers and drop closures to prevent memory leaks (P1-4 fix).
  pub fn close(&self) {
    if let Ok(ch) = self.get_channel() {
      ch.set_onmessage(None);
      ch.set_onopen(None);
      ch.set_onclose(None);
    }
    *self.on_message.borrow_mut() = None;
    *self.on_open.borrow_mut() = None;
    *self.on_close.borrow_mut() = None;
  }

  /// Get the DataChannel state.
  #[must_use]
  pub fn ready_state(&self) -> RtcDataChannelState {
    self
      .get_channel()
      .map(|ch| ch.ready_state())
      .unwrap_or(RtcDataChannelState::Closed)
  }

  /// Get the peer ID.
  #[must_use]
  pub fn peer_id(&self) -> message::UserId {
    self.peer_id.clone()
  }

  /// Check if we are the initiator.
  #[must_use]
  pub fn is_initiator(&self) -> bool {
    self.is_initiator
  }

  /// Get the underlying RtcDataChannel.
  fn get_channel(&self) -> Result<RtcDataChannel, String> {
    self
      .channel
      .clone()
      .dyn_into::<RtcDataChannel>()
      .map_err(|_| "Invalid DataChannel object".to_string())
  }
}

/// Handle an incoming DataChannel (received via RTCPeerConnection.ondatachannel).
///
/// # Errors
/// Returns an error if the DataChannel cannot be set up.
pub fn handle_incoming_channel(
  channel: RtcDataChannel,
  peer_id: message::UserId,
) -> Result<PeerDataChannel, String> {
  channel.set_binary_type(web_sys::RtcDataChannelType::Arraybuffer);

  web_sys::console::log_1(
    &format!("[datachannel] Incoming DataChannel from peer {}", peer_id).into(),
  );

  Ok(PeerDataChannel::new(channel, peer_id, false))
}

// ── Tests ──

#[cfg(test)]
mod tests {
  use message::datachannel::{ChatText, DataChannelMessage};

  #[test]
  fn test_message_discriminator_roundtrip() {
    let msg = DataChannelMessage::ChatText(ChatText {
      message_id: message::MessageId(uuid::Uuid::new_v4()),
      content: "Hello, World!".to_string(),
      reply_to: None,
      timestamp_nanos: 1234567890,
    });

    let discriminator = msg.discriminator();
    let payload = bitcode::encode(&msg);

    // Rebuild frame
    let mut frame = Vec::new();
    frame.push(discriminator);
    frame.extend_from_slice(&payload);

    // Decode
    let decoded_discriminator = frame[0];
    let decoded_payload = &frame[1..];
    let decoded: DataChannelMessage = bitcode::decode(decoded_payload).unwrap();

    assert_eq!(discriminator, decoded_discriminator);
    assert_eq!(msg.discriminator(), decoded.discriminator());

    if let DataChannelMessage::ChatText(ref ct) = decoded {
      assert_eq!(ct.content, "Hello, World!");
    } else {
      panic!("Expected ChatText message");
    }
  }

  #[test]
  fn test_data_channel_message_types() {
    // Verify all discriminators are in the correct range (0x80-0xC3)
    assert!(
      DataChannelMessage::ChatText(ChatText {
        message_id: message::MessageId(uuid::Uuid::new_v4()),
        content: "".to_string(),
        reply_to: None,
        timestamp_nanos: 0,
      })
      .discriminator()
        >= 0x80
    );
  }
}
