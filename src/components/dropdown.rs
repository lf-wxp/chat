use stylist::{self, style};
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;
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
  let click = props.onclick.clone();

  let onclick = Callback::from(move |e: MouseEvent| {
    let li = e.target().and_then(|t| t.dyn_into::<HtmlElement>().ok());
    if let Some(li) = li {
      let value = li.dataset().get("value").unwrap_or("".to_string());
      click.emit(value);
    }
  });

  html! {
    {for props.children.iter().map(|child| {
      let child = append_vnode_attr(child, "class", class_name.clone());
      add_child(child, html!{
        <section class="dropdown-content">
          <ul>
          {for props.options.iter().map(|item| {
            html!{
              <li data-value={item.value.clone()} onclick={onclick.clone()}>{item.label.clone()}</li>
            }
          })}
          </ul>
        </section>
      })
    })}
  }
}

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
        border-left: 5px solid transparent;
        border-right: 5px solid transparent;
        border-top: 5px solid rgba(var(--theme-color-rgb), 0.5);
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
