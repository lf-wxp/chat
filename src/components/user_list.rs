use bounce::use_atom_value;
use gloo_console::log;
use stylist::{self, style};
use yew::prelude::*;

use crate::{
  components::{Avatar, Dropdown},
  model::Option,
  store::{FilterWord, Users},
  utils::style,
};

#[function_component]
pub fn UserList() -> Html {
  let class_name = get_class_name();
  let users = use_atom_value::<Users>();
  let filter_word = use_atom_value::<FilterWord>();

  let options = vec![
    Option {
      value: "voice".to_string(),
      label: "语音通话".to_string(),
    },
    Option {
      value: "video".to_string(),
      label: "视频通话".to_string(),
    },
  ];

  let onclick = Callback::from(move |x: String| {
    log!("the click", x);
  });

  html! {
    <div class={class_name}>
      { for users.group_with_alphabet(filter_word.0.clone()).iter().filter(|item| item.users.len() > 0).map(|item| {
        html!{
          <section class="user-group">
              <header class="group-tag">
                {item.letter.clone()}
              </header>
              <div class="user-list">
              { for item.users.iter().map(|x| {
                html! {
                  <Dropdown options={options.clone()} onclick={onclick.clone()}>
                    <div class="user">
                      <Avatar name={x.name.clone()} />
                      <span class="user-name">{x.name.clone()}</span>
                    </div>
                  </Dropdown>
                }
              })}
            </div>
          </section>
        }
      })}
    </div>
  }
}

fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      display: flex;
      flex-flow: column nowrap;
      block-size: 100%;
      avatar {
       margin-inline-end: 10px;
      }
      .user-group {
        margin-block: 10px;
      }
      .group-tag {
        text-transform: capitalize;
        font-size: 16px;
        margin-inline-start: 5px;
        color: var(--font-color);
      }
      .user {
        display: flex;
        align-items: center;
        margin-block: 20px;
        block-size: 50px;
        padding-block: 5px;
        padding-inline: 5px;
        transition: background 0.2s ease;
        cursor: pointer;
        border-radius: var(--radius);
      }
      .user:hover {
        background: rgba(var(--theme-color-rgb), 0.5);
      }
      .user-name {
        color: var(--font-color);
        font-size: 14px;
        block-size: 100%;
        flex: 1 1 auto;
        line-height: 40px;
      }
    "#
  ))
}
