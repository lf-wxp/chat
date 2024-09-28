use bounce::use_atom_value;
use stylist::{self, style};
use yew::prelude::*;

use crate::{
  components::{Avatar, AvatarMultitude},
  store::{Chat, Chats},
  utils::{get_client_execute, style},
};

#[derive(Properties, PartialEq)]
pub struct Props {
  #[prop_or_default]
  pub keyword: String,
}

#[function_component]
pub fn ChatList(props: &Props) -> Html {
  let class_name = get_class_name();
  let chats = use_atom_value::<Chats>();
  let onclick = Callback::from(move |item: Chat| {
    get_client_execute(Box::new(|client| Box::pin(async move {})));
  });

  html! {
    <section class={class_name}>
      <div class="chat-list">
      { for chats.0.iter().filter(|x| x.filter(&props.keyword)).map(|item| {
        let item_clone = item.clone();
        let name = match item {
          Chat::Single(chat_single) => chat_single.user.name.clone(),
          Chat::Group(chat_group) => chat_group.name.clone(),
        };
        html!{
          <div class="chat-item">
            <div class="chat"
              onclick={onclick.reform(move |_| item_clone.clone())}
            >
              if let Chat::Group(item) = item {
                <AvatarMultitude names={item.users.clone().iter().map(|x| x.name.clone()).collect::<Vec<String>>()} />
              }
              if let Chat::Single(item) = item {
                <Avatar name={item.user.name.clone()} />
              }
              <span class="chat-name">{name.clone()}</span>
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
