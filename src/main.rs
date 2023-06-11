use bounce::BounceRoot;
use yew::prelude::*;

use crate::components::{Background, Side, Text};

mod components;
mod store;
mod utils;

#[function_component]
fn App() -> Html {
  html! {
    <BounceRoot>
      <section>
        <div>
          <Side />
        </div>
        <div>
          <Background />
          <Text />
        </div>
      </section>
    </BounceRoot>
  }
}

fn main() {
  yew::Renderer::<App>::new().render();
}
