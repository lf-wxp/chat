use js_sys::ArrayBuffer;

#[derive(PartialEq, Clone, Debug)]
pub enum Message {
  Text(String),
  Audio(ArrayBuffer),
  File(ArrayBuffer),
}

#[derive(PartialEq, Clone, Debug)]
pub enum MessageState {
  Pending,
  Success,
  Fail,
}

#[derive(PartialEq, Clone)]
pub enum MessageAlignment {
  Left,
  Right,
}
