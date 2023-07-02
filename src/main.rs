use bounce::BounceRoot;
use stylist::{self, style};
use yew::prelude::*;
use yew_router::{BrowserRouter, Switch};

use components::{Background, Side};
use route::{switch, Route};
use utils::style;

mod components;
mod page;
mod route;
mod store;
mod utils;
mod hook;
mod model;

#[function_component]
fn App() -> Html {
  let class_name = get_class_name();

  html! {
    <BrowserRouter>
      <BounceRoot>
        <section class={class_name}>
          <Background />
          <div class={"side"}>
            <Side />
          </div>
          <div class="content">
            <Switch<Route> render={switch}/>
          </div>
        </section>
      </BounceRoot>
    </BrowserRouter>
  }
}

fn get_class_name() -> String {
  style::get_class_name(style!(
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
  ))
}

fn main() {
  yew::Renderer::<App>::new().render();
}
