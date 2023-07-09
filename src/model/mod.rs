use js_sys::ArrayBuffer;

#[derive(PartialEq, Clone)]
pub struct Option<T = String> {
  pub label: String,
  pub value: T,
}

#[derive(PartialEq, Clone, Debug)]
pub enum Message {
  Text(String),
  Audio(ArrayBuffer),
  File(ArrayBuffer),  
}

#[derive(PartialEq, Clone)]
pub enum MessageAlignment {
  Left,
  Right,
}
