use std::{cell::RefCell, rc::Rc};

use js_sys::{ArrayBuffer, JsString};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{BinaryType::Arraybuffer, Blob, ErrorEvent, MessageEvent, WebSocket};

pub struct Websocket {
  ws: WebSocket,
  onmessage: Option<Box<dyn Fn(SocketMessage)>>,
  onerror: Option<Box<dyn Fn(ErrorEvent)>>,
  onopen: Option<Box<dyn Fn()>>,
  onclose: Option<Box<dyn Fn()>>,
  this: Option<Rc<RefCell<Self>>>,
  pending_message: Vec<SocketMessage>,
  is_connected: bool,
}

#[derive(Debug, Clone)]
pub enum SocketMessage {
  Buffer(ArrayBuffer),
  Blob(Blob),
  Str(JsString),
  None,
}

impl Websocket {
  pub fn new(url: &str) -> Result<Rc<RefCell<Self>>, JsValue> {
    let ws = WebSocket::new(url)?;
    ws.set_binary_type(Arraybuffer);
    let client = Rc::new(RefCell::new(Websocket {
      ws,
      onmessage: None,
      onerror: None,
      onopen: None,
      onclose: None,
      this: None,
      pending_message: vec![],
      is_connected: false,
    }));
    client.borrow_mut().this = Some(client.clone());
    client.borrow().bind_event();
    Ok(client)
  }

  pub fn parseMessageEvent(e: MessageEvent) -> SocketMessage {
    if let Ok(buffer) = e.data().dyn_into::<ArrayBuffer>() {
      return SocketMessage::Buffer(buffer);
    }
    if let Ok(blob) = e.data().dyn_into::<Blob>() {
      return SocketMessage::Blob(blob);
    }
    if let Ok(txt) = e.data().dyn_into::<JsString>() {
      return SocketMessage::Str(txt);
    }
    SocketMessage::None
  }

  pub fn bind_event(&self) {
    if let Some(client) = &self.this {
      let client_message = client.clone();
      let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
        if let Some(onmessage) = &client_message.borrow().onmessage {
          let message = Websocket::parseMessageEvent(e);
          onmessage(message);
        }
      });
      self
        .ws
        .set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
      onmessage_callback.forget();

      let client_error = client.clone();
      let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
        client_error.borrow_mut().is_connected = false;
        if let Some(onerror) = &client_error.borrow().onerror {
          onerror(e);
        }
      });
      self
        .ws
        .set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
      onerror_callback.forget();

      let client_open = client.clone();
      let onopen_callback = Closure::<dyn FnMut(_)>::new(move |_: MessageEvent| {
        client_open.borrow_mut().is_connected = true;
        client_open.borrow_mut().consume_pending_message();
        if let Some(onopen) = &client_open.borrow().onopen {
          onopen();
        }
      });
      self
        .ws
        .set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
      onopen_callback.forget();

      let client = client.clone();
      let onclose_callback = Closure::<dyn FnMut(_)>::new(move |_: MessageEvent| {
        client.borrow_mut().is_connected = false;
        if let Some(onclose) = &client.borrow().onclose {
          onclose();
        }
      });
      self
        .ws
        .set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
      onclose_callback.forget();
    }
  }

  pub fn set_onmessage(&mut self, callback: Box<dyn Fn(SocketMessage)>) {
    self.onmessage = Some(callback);
  }
  pub fn set_onopen(&mut self, callback: Box<dyn Fn()>) {
    self.onopen = Some(callback);
  }
  pub fn set_onerror(&mut self, callback: Box<dyn Fn(ErrorEvent)>) {
    self.onerror = Some(callback);
  }
  pub fn set_onclose(&mut self, callback: Box<dyn Fn()>) {
    self.onclose = Some(callback);
  }
  pub fn get_ws(&self) -> &WebSocket {
    &self.ws
  }
  pub fn send(&mut self, message: SocketMessage) -> Result<(), JsValue> {
    if !self.is_connected {
      self.pending_message.push(message);
      return Ok(());
    }
    match message {
      SocketMessage::Buffer(buffer) => self.ws.send_with_array_buffer(&buffer),
      SocketMessage::Blob(blob) => self.ws.send_with_blob(&blob),
      SocketMessage::Str(str) => self.ws.send_with_str(&str.as_string().unwrap()),
      _ => Ok(()),
    }
  }
  fn consume_pending_message(&mut self) {
    if self.is_connected & !self.pending_message.is_empty() {
      self.pending_message.clone().into_iter().for_each(|message| {
        let _ = self.send(message);
      });
      self.pending_message.clear();
    }
  }
}
