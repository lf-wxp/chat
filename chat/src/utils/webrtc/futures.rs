use std::{
  cell::RefCell,
  pin::Pin,
  rc::Rc,
  task::{Context, Poll, Waker},
};

use futures::{channel::mpsc, ready, Sink, SinkExt, Stream, StreamExt};
use futures_channel::mpsc::TrySendError;
use gloo_console::log;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
  HtmlMediaElement, MediaStream, RtcDataChannel, RtcDataChannelEvent, RtcIceCandidate,
  RtcIceConnectionState, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType,
  RtcSessionDescriptionInit, RtcTrackEvent, MessageEvent,
};

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
    // to-do use macro to simplify event binding code
    self.bind_ontrack()?;
    self.bind_ondatachannel()?;
    self.bind_onicecandidate()?;
    Ok(())
  }

  fn bind_ontrack(&self) -> Result<(), JsValue> {
    let track_callback = {
      let sender = self.message_sender.clone();
      Closure::<dyn FnMut(_)>::new(move |ev: RtcTrackEvent| {
        log!("ontrack", ev.track());
        let _ = sender.unbounded_send(ChannelMessage::TrackEvent(ev));
      })
    };
    self
      .peer
      .add_event_listener_with_callback("track", track_callback.as_ref().unchecked_ref())
  }

  fn bind_ondatachannel(&self) -> Result<(), JsValue> {
    let datachannel_callback = {
      let sender = self.message_sender.clone();
      Closure::<dyn FnMut(_)>::new(move |ev: RtcDataChannelEvent| {
        log!("ondatachanel", &ev);
        let _ = sender.unbounded_send(ChannelMessage::DataChannelEvent(ev));
      })
    };
    self
      .peer
      .add_event_listener_with_callback(
        "datachannel",
        datachannel_callback.as_ref().unchecked_ref(),
      )
  }

  fn bind_onicecandidate(&self) -> Result<(), JsValue>{
    let icecandidate_callback = {
      let sender = self.message_sender.clone();
      Closure::<dyn FnMut(_)>::new(move |ev: RtcPeerConnectionIceEvent| {
        let _ = sender.unbounded_send(ChannelMessage::IceEvent(ev));
      })
    };
    self
      .peer
      .add_event_listener_with_callback(
        "icecandidate",
        icecandidate_callback.as_ref().unchecked_ref(),
      )
  }

  fn bind_ondatachannel_message(&self) -> Result<(), JsValue>{
    let message_callback = {
      let sender = self.message_sender.clone();
      Closure::<dyn FnMut(_)>::new(move |ev: MessageEvent| {
        let _ = sender.unbounded_send(ChannelMessage::DataChannelMessage(ev));
      })
    };
    self
      .data_channel
      .add_event_listener_with_callback(
        "message",
        message_callback.as_ref().unchecked_ref(),
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
