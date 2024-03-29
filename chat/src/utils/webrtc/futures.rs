use std::{
  cell::RefCell,
  pin::Pin,
  rc::Rc,
  task::{Context, Poll, Waker},
};

use futures::{channel::mpsc, ready, Sink, Stream, StreamExt};
use futures_channel::mpsc::TrySendError;

use wasm_bindgen::{prelude::Closure, JsCast, JsValue};

use web_sys::{
  RtcDataChannel, RtcDataChannelEvent,
  RtcIceConnectionState, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcTrackEvent, MessageEvent,
};

use crate::bind_event;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct WebRTC {
  peer: RtcPeerConnection,
  data_channel: RtcDataChannel,
  waker: Rc<RefCell<Option<Waker>>>,
  message_receiver: mpsc::UnboundedReceiver<ChannelMessage>,
  message_sender: mpsc::UnboundedSender<ChannelMessage>,
}

#[derive(Debug)]
pub enum ChannelMessage {
  ErrorEvent,
  TrackEvent(RtcTrackEvent),
  DataChannelEvent(RtcDataChannelEvent),
  IceEvent(RtcPeerConnectionIceEvent),
  DataChannelCloseEvent,
  DataChannelErrorEvent,
  DataChannelMessage(MessageEvent),
}

impl WebRTC {
  pub fn new() -> Result<Self, JsValue> {
    let (sender, receiver) = mpsc::unbounded();
    let peer = RtcPeerConnection::new()?;
    let data_channel = peer.create_data_channel("chat");
    let rtc = Self {
      peer,
      data_channel,
      waker: Rc::new(RefCell::new(None)),
      message_receiver: receiver,
      message_sender: sender,
    };
    rtc.setup()?;
    Ok(rtc)
  }

  fn setup(&self) -> Result<(), JsValue> {
    self.bind_ontrack()?;
    self.bind_ondatachannel()?;
    self.bind_onicecandidate()?;
    Ok(())
  }

  fn bind_ontrack(&self) -> Result<(), JsValue> {
    bind_event!(
      self.peer,
      "track",
      self.message_sender,
      ChannelMessage::TrackEvent,
      RtcTrackEvent
    )
  }

  fn bind_ondatachannel(&self) -> Result<(), JsValue> {
    bind_event!(
      self.peer,
      "datachannel",
      self.message_sender,
      ChannelMessage::DataChannelEvent,
      RtcDataChannelEvent
    )
  }

  fn bind_onicecandidate(&self) -> Result<(), JsValue>{
    bind_event!(
      self.peer,
      "icecandidate",
      self.message_sender,
      ChannelMessage::IceEvent,
      RtcPeerConnectionIceEvent
    )
  }

  fn bind_ondatachannel_message(&self) -> Result<(), JsValue>{
    bind_event!(
      self.data_channel,
      "message",
      self.message_sender,
      ChannelMessage::DataChannelMessage,
      MessageEvent
    )
  }

  pub fn state(&self) -> RtcIceConnectionState {
    self.peer.ice_connection_state()
  }
}

impl Sink<ChannelMessage> for WebRTC {
  type Error = TrySendError<ChannelMessage>;

  fn start_send(self: Pin<&mut Self>, item: ChannelMessage) -> Result<(), Self::Error> {
    let this = self.get_mut();
    println!("start_send");
    this
      .message_sender
      .unbounded_send(item)
  }

  fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    if true {
      Poll::Ready(Ok(()))
    } else {
      *self.waker.borrow_mut() = Some(cx.waker().clone());
      Poll::Pending
    }
  }

  fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }

  fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }
}

impl Stream for WebRTC {
  type Item = ChannelMessage;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.get_mut();
    println!("poll_next before");
    let msg = ready!(this.message_receiver.poll_next_unpin(cx));
    println!("poll_next  after {:?}", &msg);
    Poll::Ready(msg)
  }
}
