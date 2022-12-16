use yew::prelude::*;
use web_sys::{ console, MouseEvent };
use yew:: Callback;
use gloo_console::log;

#[function_component(App)]
fn app() -> Html {
  console::log_1(&"hello world".into());

  let onclick =  Callback::from(|e: MouseEvent | {
    log!("the event is ", e);
  });
  html! {
    <>
      <h1>{ "Hello World" }</h1>
      <button {onclick}>{ "click"}</button>
    </>
  }
}

fn main() {
  yew::Renderer::<App>::new().render();
}
