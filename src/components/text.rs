use gloo_console::log;
use web_sys::MouseEvent;
use yew::prelude::*;
use yew::Callback;
use stylist::{self, style};

#[function_component]
pub fn Text() -> Html {
  let onclick = Callback::from(|e: MouseEvent| {
    log!("the event is ", e);
  });

  let class_name = get_style().unwrap_or_default();

  html! {
    <>
      <h1 class={class_name}>{"hello world"}</h1>
      <button {onclick}>{ "click hello"}</button>
    </>
  }
}

fn get_style() -> Result<String, stylist::Error> {
  Ok(style!(
    // A CSS string literal
    r#"
      background-color: red;
      color: blue;
    "#
  )?.get_class_name().to_owned())

}
