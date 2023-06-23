use bounce::use_atom_value;
use gloo_console::log;
use stylist::{self, style};

use yew::prelude::*;

use crate::components::Avatar;
use crate::store::{Users};
use crate::utils::style;

#[function_component]
pub fn UserList() -> Html {
  let class_name = get_class_name();
  let users = use_atom_value::<Users>();
  let users_clone = users.clone();

  use_effect(move || {
    let d = users_clone.group_with_alphabet();
    log!("pingyin", format!("{:?}", d));
  });
  html! {
    <div class={class_name}>
      { for users.0.iter().map(|item| {
        html!{
          <div class={"user"}>
            <Avatar name={item.name.clone()} />
            <span class={"user-name"}>{item.name.clone()}</span>
          </div>
        }
      })}
    </div>
  }
}

fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      display:flex;
      inline-size: 300px;
      flex-flow: column wrap;
      > avatar {
       margin: 5px;
      }
      .user {
      }
    "#
  ))
}
