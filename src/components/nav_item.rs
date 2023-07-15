use stylist::style;
use yew::{prelude::*, Properties};
use yew_icons::{Icon, IconId};
use yew_router::prelude::{use_navigator, use_route};

use crate::{route::Route, utils::style};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub icon: IconId,
  pub route: Route,
}

#[function_component]
pub fn NavItem(props: &Props) -> Html {
  let navigator = use_navigator().unwrap();
  let current_route: Route = use_route().unwrap();
  let route = props.route.clone();
  let active_name = if current_route == route { "active"} else {""};
  let class_name = format!("{} {}", get_class_name(), active_name);
  let icon_click = Callback::from(move |_| {
    navigator.push(&route);
  });

  html! {
      <span class={class_name} onclick={icon_click}>
        <Icon  icon_id={props.icon} width="16px" height="16px" />
      </span>
    }
}

fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        inline-size: 30px;
        block-size: 30px;
        color: #8896a4;
        font-size: 16px;
        display: flex;
        flex-flow: column nowrap;
        justify-content: center;
        align-items: center;
        cursor: pointer;
        border-radius: var(--radius);
        transition: all 0.2s ease;
        margin-block: 10px;

      svg {
        inline-size: 16px;
        block-size: 16px;
      }
      
      :hover, &.active {
        background: var(--theme-color);
        color: #51b66d;
      }
    "#
  ))
}
