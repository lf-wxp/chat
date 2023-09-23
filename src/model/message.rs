use js_sys::ArrayBuffer;
use web_sys::{Blob, File, Url};

use crate::utils::{array_buffer_to_blob_url, read_file, blob_to_array_buffer};

#[derive(PartialEq, Clone, Debug)]
pub enum MessageBinary {
  Buffer(ArrayBuffer),
  File(File),
  Blob(Blob),
}

impl MessageBinary {
  pub async fn get_url(&self) -> String {
    match self {
      MessageBinary::Blob(blob) => {
        Url::create_object_url_with_blob(blob).unwrap_or("".to_string())
      }
      MessageBinary::Buffer(buffer) => {
        array_buffer_to_blob_url(buffer, "").unwrap_or("".to_string())
      }
      MessageBinary::File(file) => {
        let buffer = read_file(file.clone()).await.unwrap();
        array_buffer_to_blob_url(&buffer, "").unwrap_or("".to_string())
      }
    }
  }
  pub async fn get_buffer(&self) -> ArrayBuffer {
    match self {
      MessageBinary::Blob(blob) => {
        blob_to_array_buffer(blob).await.unwrap()
      },
      MessageBinary::Buffer(buffer) => buffer.clone(), 
      MessageBinary::File(file) => {
        read_file(file.clone()).await.unwrap()
      }
    }
  }
}

#[derive(PartialEq, Clone, Debug)]
pub enum Message {
  Text(String),
  Audio(MessageBinary),
  File(MessageBinary),
  Image(MessageBinary),
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
