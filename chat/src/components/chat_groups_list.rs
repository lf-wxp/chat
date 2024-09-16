use bounce::use_atom_value;
use stylist::{self, style};
use yew::prelude::*;

use crate::{
  components::AvatarMultitude,
  store::{ChatGroup, ChatGroups},
  utils::{get_client_execute, style},
};

#[function_component]
pub fn ChatGroupsList() -> Html {
  let class_name = get_class_name();
  let groups = use_atom_value::<ChatGroups>();
  let onclick = Callback::from(move |item: ChatGroup| {
    get_client_execute(Box::new(|client| {
      Box::pin(async move {
      })
    }));
  });

  html! {
    <section class={class_name}>
      { for groups.0.iter().map(|item| {
        let item_clone = item.clone();
        html!{
          <div class="chat-list">
            <div class="chat-item">
              <div class="chat"
                onclick={onclick.reform(move |_| item_clone.clone())}
              >
                <AvatarMultitude name={item.name.clone()} />
                <span class="chat-name">{item.name.clone()}</span>
              </div>
            </div>
        </div>
        } 
      })}
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
      avatar {
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
