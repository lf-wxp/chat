use yew::prelude::*;

use crate::components::{ Text, Background };

mod components;
mod utils;

#[function_component]
fn App() -> Html {
  html! {
    <>
      <Background />
    </>
  }
}

fn main() {
  yew::Renderer::<App>::new().render();
}
