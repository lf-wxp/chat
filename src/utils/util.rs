use futures::channel::oneshot;
use gloo_console::log;
use indexmap::{self, IndexMap};
use js_sys::{ArrayBuffer, Uint8Array};
use rand::{self, Rng};
use std::{cell::RefCell, ops::Range, rc::Rc};
use unicode_segmentation::UnicodeSegmentation;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{window, FileReader, HtmlTextAreaElement, Window, Blob, Url, BlobPropertyBag};
use yew::{
  virtual_dom::{ApplyAttributeAs, Attributes, VNode},
  AttrValue,
};
use base64::{Engine as _, engine::general_purpose};


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
    x.map_or(None, |x| {
      Some(get_correct_selection_start(value, x).try_into().unwrap())
    })
  })
}

pub fn get_textarea_selection_offset(html: HtmlTextAreaElement, value: &str) -> Selection {
  Selection {
    start: get_selection_offset(html.selection_start(), value),
    end: get_selection_offset(html.selection_end(), value),
  }
}

pub async fn read_file(file: web_sys::File) -> Result<js_sys::ArrayBuffer, JsValue> {
  let file_reader = FileReader::new().unwrap();
  let file_reader_clone = file_reader.clone();
  let (tx, rx) = oneshot::channel::<Result<JsValue, JsValue>>();
  let tx = Rc::new(RefCell::new(Some(tx)));

  {
    let tx = tx.clone();
    let file_reader_ok = file_reader.clone();
    let onload = Closure::once(move || {
      let result = file_reader_ok.result().unwrap();
      let tx_opt = tx.borrow_mut().take();
      if let Some(tx) = tx_opt {
        let _ = tx.send(Ok(result));
      }
    });

    file_reader.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();
  }

  {
    let tx = tx.clone();
    let file_reader_none = file_reader.clone();
    let onerror = Closure::once(move || {
      let error = file_reader_none.error().unwrap();
      let tx_opt = tx.borrow_mut().take();
      if let Some(tx) = tx_opt {
        let _ = tx.send(Err(error.into()));
      }
    });

    file_reader_clone.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();
  }

  file_reader.read_as_array_buffer(&file)?;
  let result = rx
    .await
    .map_err(|_| JsValue::from_str("oneshot channel canceled"))?;
  result.map(|v| v.dyn_into().unwrap())
}

pub fn create_image_url(array_buffer: &ArrayBuffer, mime_type: &str) -> Result<String, JsValue> {
  let blob = Blob::new_with_u8_array_sequence_and_options(
    &Uint8Array::new(array_buffer),
    BlobPropertyBag::new().type_(mime_type),
  )?;

  let url = Url::create_object_url_with_blob(&blob)?;
  log!("url", url.clone());
  Ok(url)
} 

pub fn create_base64_string(array_buffer: &ArrayBuffer) -> String {
  let uint8_array = Uint8Array::new(&array_buffer);
  let length = uint8_array.length() as usize;
  let mut vec = vec![0; length];
  uint8_array.copy_to(&mut vec);

  let base64 = general_purpose::STANDARD.encode(&vec);
  format!("data:image/png;base64,{}", base64)
}
