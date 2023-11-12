use std::{cell::RefCell, rc::Rc};

use crate::{
  CastMessage, Channel, RequestMessage, ResponseMessage, SdpMessage, SdpType, SignalSend, Transmit,
  TransmitMessage, Unicast,
};

type Callback = Rc<RefCell<Option<Box<dyn Fn(String)>>>>;

type TripleCallback = (
  Box<dyn Fn(String)>,
  Box<dyn Fn(String)>,
  Box<dyn Fn(String)>,
);

pub struct SignalChannel<T: Channel> {
  sender: String,
  receiver: String,
  channel: Rc<RefCell<T>>,
  receive_offer: Callback,
  receive_answer: Callback,
  receive_ice: Callback,
}
impl<T: Channel> SignalChannel<T> {
  pub fn new(sender: String, receiver: String, channel: Rc<RefCell<T>>) -> Self {
    let mut signal_channel = SignalChannel {
      sender,
      receiver,
      channel,
      receive_offer: Rc::new(RefCell::new(None)),
      receive_answer: Rc::new(RefCell::new(None)),
      receive_ice: Rc::new(RefCell::new(None)),
    };
    signal_channel.bind_event();
    signal_channel
  }

  fn send_message(&mut self, message: CastMessage) {
    let action = &RequestMessage::Transmit(Transmit::Unicast(Unicast {
      from: self.sender.clone(),
      to: self.receiver.clone(),
      message,
    }));
    let message = serde_json::to_string(action).unwrap();
    self.channel.borrow_mut().send(&message);
  }

  fn deal_callback(&self) -> TripleCallback {
    let offer_clone = self.receive_offer.clone();
    let receive_offer = Box::new(move |sdp: String| {
      if let Some(callback) = offer_clone.borrow().as_ref() {
        callback(sdp);
      }
    });
    let answer_clone = self.receive_answer.clone();
    let receive_answer = Box::new(move |sdp: String| {
      if let Some(callback) = answer_clone.borrow().as_ref() {
        callback(sdp);
      }
    });
    let ice_clone = self.receive_ice.clone();
    let receive_ice = Box::new(move |ice: String| {
      if let Some(callback) = ice_clone.borrow().as_ref() {
        callback(ice);
      }
    });

    (receive_offer, receive_answer, receive_ice)
  }

  fn bind_event(&mut self) {
    let (receive_offer, receive_answer, receive_ice) = self.deal_callback();
    let onmessage = Box::new(move |msg: &str| {
      if let Ok(ResponseMessage::Transmit(TransmitMessage { message, .. })) =
        serde_json::from_str::<ResponseMessage>(msg)
      {
        if let CastMessage::Sdp(SdpMessage { sdp, sdp_type }) = message.clone() {
          match sdp_type {
            SdpType::Offer => receive_offer(sdp),
            SdpType::Answer => receive_answer(sdp),
          }
        }
        if let CastMessage::Ice(ice) = message {
          receive_ice(ice)
        }
      }
    });
    self.channel.borrow_mut().onmessage(onmessage);
  }

  pub fn set_callback(&self, (offer, answer, ice): TripleCallback) {
    *self.receive_offer.borrow_mut() = Some(offer);
    *self.receive_answer.borrow_mut() = Some(answer);
    *self.receive_answer.borrow_mut() = Some(ice);
  }
}

impl<T: Channel> SignalSend for SignalChannel<T> {
  fn send_offer(&mut self, sdp: String) {
    let message = CastMessage::Sdp(SdpMessage {
      sdp_type: SdpType::Offer,
      sdp,
    });
    self.send_message(message);
  }
  fn send_answer(&mut self, sdp: String) {
    let message = CastMessage::Sdp(SdpMessage {
      sdp_type: SdpType::Answer,
      sdp,
    });
    self.send_message(message);
  }
  fn send_ice(&mut self, ice: String) {
    let message = CastMessage::Ice(ice);
    self.send_message(message);
  }
}
