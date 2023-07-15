use yew::prelude::*;
use web_sys::HtmlElement;
use wasm_bindgen::JsCast;
use yew_hooks::use_event_with_window;

#[hook]
pub fn use_click_exclusive<F>(class_set: Vec<String>, callback: F)
where
  F: Fn() + 'static,
{
  use_event_with_window("click", move |e: MouseEvent| {
    let target = e.target().and_then(|t| t.dyn_into::<HtmlElement>().ok());
    if let Some(target) = target {
      let is_contain = class_set.iter().any(|x| target.closest(x).map_or(false, |x| x.is_some()));
      if !is_contain {
        callback();
      }
    }
  });
}
