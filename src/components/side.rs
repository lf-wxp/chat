use gloo_console::log;
use stylist::{self, style};
use web_sys::MouseEvent;
use yew::prelude::*;
use yew::Callback;
use yew_icons::{Icon, IconId};

use crate::utils::style;

#[function_component]
pub fn Side() -> Html {
  let class_name = get_class_name();

  html! {
    <side class={class_name}>
      <div class={"side-nav"}>
        <span class={"icon"}>
          <Icon icon_id={IconId::HeroiconsSolidUserGroup} width={"16px"} height={"16px"}/>
        </span>
      </div>
    </side>
  }
}

fn get_class_name() -> String {
  style::get_class_name(
    style!(
      // A CSS string literal
      r#"
        inline-size: 100%;
        block-size: 100%;
        display: flex;
        flex-flow: column nowrap;
        justify-content: center;
        align-items: center;

        .side-nav {
          display: flex;
          flex-flow: column nowrap;
          justify-content: center;
          align-items: center;
        }

        .icon {
          inline-size: 30px;
          block-size: 30px;
          color: #8896a4;
          font-size: 16px;
          display: flex;
          flex-flow: column nowrap;
          justify-content: center;
          align-items: center;
          cursor: pointer;
          border-radius: 30%;
          transition: all 0.2s ease;
        }

        .icon svg {
          inline-size: 16px;
          block-size: 16px;
        }
        
        .icon:hover {
          background: var(--theme-color);
          color: #51b66d;
        }
    "#
    )
  )
}
