use bounce::use_atom_value;
use stylist::{self, style};
use yew::prelude::*;

use crate::{components::Avatar, store::User, utils::style};

#[function_component]
pub fn Account() -> Html {
  let class_name = get_class_name();
  let user = use_atom_value::<User>();

  html! {
    <>
      <div class={class_name}>
        <Avatar name={user.name.clone()} />
        <span class="user-name">{user.name.clone()}</span>
      </div>
    </>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      inline-size: 30px;
      block-size: 30px;
      position: relative;
      cursor: pointer;
      :hover .user-name {
        visibility: visible;
        opacity: 1;
      }
      avatar {
        inline-size: inherit;
        block-size: inherit;
      }
      .user-name {
        position: absolute;        
        visibility: hidden;
        opacity: 0;
        inset-block-start: 0; 
        inset-block-end: 0; 
        margin: auto;
        padding: 4px 8px;
        border-radius: calc(var(--radius) / 3);
        background: var(--theme-color);
        color: var(--font-color);
        transform: translateX(5px);
        white-space: nowrap;
        line-height: 1;
        block-size: fit-content;
        transition: all .2s ease;
      }
      .user-name::before {
        content: "";
        position: absolute;
        inset-inline-start: -5px;
        inset-block-end: 0;
        inset-block-start: 0;
        margin: auto;
        block-size: 0;
        inline-size: 0;
        border-bottom: 5px solid transparent;
        border-top: 5px solid transparent;
        border-right: 5px solid var(--theme-color);
      }
    "#
  ))
}
