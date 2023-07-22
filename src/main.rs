use bounce::BounceRoot;
use model::ChatHistory;
use std::cell::OnceCell;
use stylist::{self, style};
use yew::prelude::*;
use yew_router::{BrowserRouter, Switch};

use components::{Background, Chat, FakeSet, Side};
use route::{switch, Route};
use utils::style;

mod components;
mod hook;
mod model;
mod page;
mod route;
mod store;
mod utils;

#[function_component]
fn App() -> Html {
  let class_name = get_class_name();

  html! {
    <BrowserRouter>
      <BounceRoot>
        <FakeSet />
        <section class={class_name}>
          <Background />
          <div class={"side"}>
            <Side />
          </div>
          <div class="content">
            <Switch<Route> render={switch}/>
          </div>
          <Chat />
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

static mut CHAT_HISTORY: OnceCell<ChatHistory> = OnceCell::new();

pub fn get_chat_history() -> Option<&'static mut ChatHistory> {
  unsafe { CHAT_HISTORY.get_mut() }
}

fn main() {
  unsafe { CHAT_HISTORY.get_or_init(|| ChatHistory::default()) };
  yew::Renderer::<App>::new().render();
}
