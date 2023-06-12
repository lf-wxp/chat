use bounce::BounceRoot;
use utils::style;
use stylist::{self, style};
use yew::prelude::*;

use components::{Background, Side, Text};

mod components;
mod store;
mod utils;

#[function_component]
fn App() -> Html {
  let class_name = get_class_name();

  html! {
    <BounceRoot>
      <section class={class_name}>
        <Background />
        <div class={"side"}>
          <Side />
        </div>
        <div class="content">
          <Text />
        </div>
      </section>
    </BounceRoot>
  }
}

fn get_class_name() -> String {
  style::get_class_name(
    style!(
      r#"
        display: flex;
        flex-flow: nowrap; 
        inline-size: 100%;
        block-size: 100%;

        .side {
          inline-size: 40px; 
          padding: var(--padding);
          backdrop-filter: blur(15px);
        }
        
        .content {
          padding: calc(var(--padding) * 2);
          flex: 1 1 auto;
        }
    "#
    )
  )
}

fn main() {
  yew::Renderer::<App>::new().render();
}
