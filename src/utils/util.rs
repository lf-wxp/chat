use base64::{engine::general_purpose, Engine as _};
use indexmap::{self, IndexMap};
use js_sys::{ArrayBuffer, Uint8Array};
use rand::{self, Rng};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, Blob, BlobPropertyBag, Event, FileReader, HtmlTextAreaElement, Url, Window, AudioContext, AudioBuffer};
use yew::{
  virtual_dom::{ApplyAttributeAs, Attributes, VNode},
  AttrValue,
};

use crate::{components::Selection, model::ChatMessage, utils::get_chat_history};

pub fn random(rang: Range<u16>) -> u16 {
  rand::thread_rng().gen_range(rang)
}

pub fn num_in_range(start: f64, end: f64, num: f64) -> f64 {
  if num <= start {
    return start;
  }
  if num >= end {
    return end;
  }
  num
}

pub fn get_window() -> Window {
  window().expect("no global `window` exists")
}

pub fn get_dpr() -> f64 {
  if let Some(w) = window() {
    return w.device_pixel_ratio();
  }
  1.0
}

pub fn request_animation_frame(f: &Closure<dyn FnMut()>) {
  get_window()
    .request_animation_frame(f.as_ref().unchecked_ref())
    .expect("should register `requestAnimationFrame` OK");
}

pub fn class_name_determine(condition: bool, name: &str, append: &str) -> String {
  format!("{} {}", name, if condition { append } else { "" })
}

pub fn get_vnode_attr(vnode: VNode, attr: &str) -> String {
  match vnode {
    VNode::VTag(vtag) => vtag
      .attributes
      .iter()
      .find(|&(key, _value)| key == attr)
      .map_or("".to_string(), |(_key, val)| val.to_string()),
    _ => "".to_string(),
  }
}

pub fn append_vnode_attr(vnode: VNode, key: &'static str, val: String) -> VNode {
  let pre_val = get_vnode_attr(vnode.clone(), key);

  match vnode {
    VNode::VTag(mut vtag) => {
      let mut indexmap = IndexMap::new();
      indexmap.insert(
        AttrValue::from(key),
        (
          AttrValue::from(format!("{} {}", pre_val, val)),
          ApplyAttributeAs::Attribute,
        ),
      );
      let attr = Attributes::IndexMap(indexmap);
      vtag.set_attributes(attr);
      VNode::VTag(vtag)
    }
    _ => vnode.clone(),
  }
}

pub fn add_child(vnode: VNode, child: VNode) -> VNode {
  match vnode {
    VNode::VTag(mut vtag) => {
      vtag.add_child(child);
      VNode::VTag(vtag)
    }
    _ => vnode.clone(),
  }
}
pub fn get_target<T, H>(e: T) -> Option<H>
where
  T: AsRef<web_sys::Event>,
  H: JsCast,
{
  e.as_ref().target().and_then(|t| t.dyn_into::<H>().ok())
}

pub fn get_history(chat: &str) -> Option<&'static mut Vec<ChatMessage>> {
  get_chat_history()
    .map(|x| &mut x.0)
    .and_then(|x| x.get_mut(chat))
}

pub fn get_correct_selection_start(s: &str, utf16_position: u32) -> usize {
  let mut visual_position = 0;
  let mut current_utf16_position = 0;
  let graphemes = UnicodeSegmentation::graphemes(s, true).collect::<Vec<&str>>();

  for c in graphemes.iter() {
    if current_utf16_position >= utf16_position as usize {
      break;
    }
    let char_utf16_len = c.encode_utf16().count();
    current_utf16_position += char_utf16_len;
    visual_position += 1;
  }

  visual_position
}

pub fn get_string_len(s: &str) -> usize {
  UnicodeSegmentation::graphemes(s, true)
    .collect::<Vec<&str>>()
    .len()
}

pub fn get_selection_offset(result: Result<Option<u32>, JsValue>, value: &str) -> Option<u32> {
  result.map_or(None, |x| {
    x.map(|x| get_correct_selection_start(value, x).try_into().unwrap())
  })
}

pub fn get_textarea_selection_offset(html: HtmlTextAreaElement, value: &str) -> Selection {
  Selection {
    start: get_selection_offset(html.selection_start(), value),
    end: get_selection_offset(html.selection_end(), value),
  }
}

pub async fn read_file(file: web_sys::File) -> Result<js_sys::ArrayBuffer, JsValue> {
  let promise = js_sys::Promise::new(&mut |resolve, reject| {
    let file_reader = FileReader::new().unwrap();
    let file_reader_ok = file_reader.clone();
    let load = Closure::wrap(Box::new(move |_event: Event| {
      let array_buffer: js_sys::ArrayBuffer = file_reader_ok.result().unwrap().dyn_into().unwrap();
      let _ = resolve.call1(&JsValue::undefined(), &array_buffer);
    }) as Box<dyn FnMut(_)>);
    let error = Closure::wrap(Box::new(move |err: JsValue| {
      let _ = reject.call1(&JsValue::undefined(), &err);
    }) as Box<dyn FnMut(_)>);
    let _ = file_reader.add_event_listener_with_callback("load", load.as_ref().unchecked_ref());
    let _ = file_reader.add_event_listener_with_callback("error", error.as_ref().unchecked_ref());
    let _ = file_reader.read_as_array_buffer(&file);
    load.forget();
    error.forget();
  });
  let array_buffer = JsFuture::from(promise).await?;
  let array_buffer: js_sys::ArrayBuffer = array_buffer.dyn_into()?;
  Ok(array_buffer)
}

pub fn array_buffer_to_blob_url(
  array_buffer: &ArrayBuffer,
  mime_type: &str,
) -> Result<String, JsValue> {
  let array: js_sys::Array = js_sys::Array::new();
  array.push(array_buffer);
  let blob =
    Blob::new_with_u8_array_sequence_and_options(&array, BlobPropertyBag::new().type_(mime_type))?;

  let url = Url::create_object_url_with_blob(&blob)?;
  Ok(url)
}

pub fn create_base64_string(array_buffer: &ArrayBuffer) -> String {
  let uint8_array = Uint8Array::new(array_buffer);
  let length = uint8_array.length() as usize;
  let mut vec = vec![0; length];
  uint8_array.copy_to(&mut vec);

  let base64 = general_purpose::STANDARD.encode(&vec);
  format!("data:image/png;base64,{}", base64)
}

pub async fn blob_to_array_buffer(blob: &Blob) -> Result<js_sys::ArrayBuffer, JsValue> {
  JsFuture::from(blob.array_buffer()).await?.dyn_into::<js_sys::ArrayBuffer>()
}

pub async fn get_duration(array_buffer: &ArrayBuffer) -> Result<f64, JsValue> {
  let audio_context = AudioContext::new()?;
  let decode_promise = audio_context.decode_audio_data(array_buffer)?;
  let audio_buffer = JsFuture::from(decode_promise).await?.dyn_into::<AudioBuffer>()?;
  let duration = audio_buffer.duration();
  Ok(duration)
}
