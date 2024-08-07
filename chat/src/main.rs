use bounce::BounceRoot;
use stylist::{self, style};
use yew::prelude::*;
use yew_i18n::I18nProvider;
use yew_router::{BrowserRouter, Switch};

use components::{
  Background, Chat, DialogProvider, FakeSet, MediaRequestProvider, NotifyProvider, Register,
  Service, Side,
};
use route::{switch, Route};
use utils::{style, TRANSLATIONS};

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
  let supported_languages = vec!["en", "zh"];

  html! {
    <BrowserRouter>
      <BounceRoot>
        <I18nProvider supported_languages={supported_languages} translations={TRANSLATIONS.clone()} >
          <MediaRequestProvider>
            <NotifyProvider>
              <DialogProvider>
                <Service />
                <FakeSet />
                <Register />
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
              </DialogProvider>
            </NotifyProvider>
          </MediaRequestProvider>
        </I18nProvider>
      </BounceRoot>
    </BrowserRouter>
  }
}

#[allow(non_upper_case_globals)]
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
          z-index: 1;
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
