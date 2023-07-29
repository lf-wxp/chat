use indexmap::{self, IndexMap};
use rand::{self, Rng};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{window, HtmlTextAreaElement, Window};
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
