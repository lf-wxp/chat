use indexmap::{self, IndexMap};
use rand::{self, Rng};
use std::ops::Range;
use wasm_bindgen::{prelude::Closure, JsCast};
use yew::{
  virtual_dom::{ApplyAttributeAs, Attributes, VNode},
  AttrValue,
};

use crate::{utils::get_chat_history, model::ChatMessage};

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

pub fn get_window() -> web_sys::Window {
  web_sys::window().expect("no global `window` exists")
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
      return VNode::VTag(vtag);
    }
    _ => vnode.clone(),
  }
}

pub fn add_child(vnode: VNode, child: VNode) -> VNode {
  match vnode {
    VNode::VTag(mut vtag) => {
      vtag.add_child(child);
      return VNode::VTag(vtag);
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
    .and_then(|x| Some(&mut x.0))
    .and_then(|x| x.get_mut(chat))
}
