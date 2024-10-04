use bounce::{use_atom_setter, use_atom_value, use_slice, use_slice_value};
use stylist::{self, style};
use yew::prelude::*;

use crate::{
  components::{Avatar, AvatarMultitude},
  store::{Chat, Chats, ChatsAction, CurrentChat, User},
  utils::style,
};

#[derive(Properties, PartialEq)]
pub struct Props {
  #[prop_or_default]
  pub keyword: String,
}

#[function_component]
pub fn ChatList(props: &Props) -> Html {
  let class_name = get_class_name();
  let chats = use_slice_value::<Chats>();
  let user = use_atom_value::<User>();
  let chats_slice = use_slice::<Chats>();
  let chat_setter = use_atom_setter::<CurrentChat>();
  let onclick = Callback::from(move |item: Chat| {
    chats_slice.dispatch(ChatsAction::Append(item.clone()));
    chat_setter(CurrentChat(Some(item)))
  });

  let render_avatar = move |users: Vec<User>| {
    let users = users
      .iter()
      .filter(|x| **x != *user)
      .map(|x| x.name.clone())
      .collect::<Vec<String>>();
    if users.len() == 1 {
      return html! {
        <Avatar name={users[0].clone()} />
      };
    }
    html! {
      <AvatarMultitude names={users} />
    }
  };

  html! {
    <section class={class_name}>
      <div class="chat-list">
      { for chats.0.iter().filter(|x| x.filter(&props.keyword)).map(|item| {
        let item_clone = item.clone();
        html!{
          <div class="chat-item">
            <div class="chat"
              onclick={onclick.reform(move |_| item_clone.clone())}
            >
              { render_avatar(item.users.clone())}
              <span class="chat-name">{item.name.clone()}</span>
            </div>
          </div>
        }
      })}
      </div>
    </section>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      display: flex;
      flex-flow: column nowrap;
      block-size: 100%;
      .chat > avatar {
       margin-inline-end: 10px;
      }
      .chat > .avatar-multi {
       margin-inline-end: 10px;
      }
      .chat-list {
        inline-size: 100%;
      }
      .chat-item {
        inline-size: 100%;
        display: flex;
        align-items: center;
        margin-block: 20px;
        transition: background 0.2s ease;
        cursor: pointer;
        border-radius: var(--radius);
      }
      .chat-item:hover {
        background: rgba(var(--theme-color-rgb), 0.5);
      }
      .chat-item:hover .chat {
        inline-size: calc(100% - var(--avatar-size, 40px) * 2);
      }
      .chat{
        display: flex;
        align-items: center;
        inline-size: 100%;
      }
      .chat-name {
        color: var(--font-color);
        font-size: 14px;
        block-size: 100%;
        flex: 0 0 auto;
        line-height: 40px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        inline-size: calc(100% - var(--avatar-size, 40px) - 10px);
      }
    "#
  ))
}
