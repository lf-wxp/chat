use stylist::{self, style};
use yew::prelude::*;

use crate::model::Option;
use crate::utils::{add_child, append_vnode_attr, style};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub options: Vec<Option>,
  pub onclick: Callback<String>,
  pub children: Children,
}

#[function_component]
pub fn Dropdown(props: &Props) -> Html {
  let class_name = get_class_name();
  let view_item = |item: &Option| {
    let onclick = props.onclick.clone();
    let call_item = item.value.to_string();
    html! {
      <li onclick={onclick.reform(move |_| call_item.clone())}>{item.label.clone()}</li>
    }
  };

  html! {
    {for props.children.iter().map(|child| {
      let child = append_vnode_attr(child, "class", class_name.clone());
      add_child(child, html!{
        <section class="dropdown-content">
          <ul>
          {for props.options.iter().map(view_item)}
          </ul>
        </section>
      })
    })}
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      position: relative;
      .dropdown-content {
        position: absolute; 
        background: rgba(var(--theme-color-rgb), 0.5);
        border-radius: calc(var(--radius) / 8);
        display: inline-block;
        color: var(--font-color);
        padding: 5px;
        transition: all 0.2s ease;
        inset-block-start: 0;
        inset-inline-start: 60px;
        transform: translateY(calc(-100% - 5px));
        font-size: 12px;
        visibility: hidden;
        opacity: 0;
      }
      .dropdown-content::before {
        content: "";
        position: absolute;
        inset-inline-start: 0;
        inset-block-end: -5px;
        block-size: 0;
        inline-size: 0;
        border-inline-start: 5px solid transparent;
        border-inline-end: 5px solid transparent;
        border-block-start: 5px solid rgba(var(--theme-color-rgb), 0.5);
      }

      :hover .dropdown-content {
        visibility: visible;
        opacity: 1;
      }
      .dropdown-content li {
        padding: 2px 5px;
        border-radius: calc(var(--radius) / 8);
        transition: all 0.2s ease;
      }
      .dropdown-content li:hover {
        background: var(--theme-color);
      }
    "#
  ))
}
