use std::time::Duration;
use bounce::use_selector_value;
use gloo_timers::future::sleep;
use stylist::{self, style};
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
  components::Avatar,
  model::Option,
  store::{ User, Users},
  utils::{get_client_execute, style, ChannelMessage},
};
#[derive(Properties, PartialEq)]
pub struct Props {
  #[prop_or_default]
  pub keyword: String,
}

#[function_component]
pub fn UserList(props: &Props) -> Html {
  let class_name = get_class_name();
  let users = use_selector_value::<Users>();

  let options = [
    Option {
      value: "voice".to_string(),
      label: "语音通话".to_string(),
      icon: Some(IconId::BootstrapTelephoneForwardFill),
    },
    Option {
      value: "video".to_string(),
      label: "视频通话".to_string(),
      icon: Some(IconId::BootstrapCameraVideoFill),
    },
  ];
  let onclick = Callback::from(move |(user, _call_type): (User, String)| {
    get_client_execute(Box::new(|client| {
      Box::pin(async move {
        // client.request_media(user.uuid, MediaType::Video).await;
      })
    }));
  });

  let ondblclick = Callback::from(move |user: User| {
    get_client_execute(Box::new(|client| {
      Box::pin(async move {
        client.request_datachannel(user.uuid.clone()).await;
      })
    }));
  });

  html! {
    <div class={class_name}>
      { for users.group_with_alphabet(props.keyword.clone()).iter().filter(|item| !item.users.is_empty()).map(|item| {
        html!{
          <section class="user-group">
              <header class="group-tag">
                {item.letter.clone()}
              </header>
              <div class="user-list">
              { for item.users.iter().map(|x| {
                let user = x.clone();
                html! {
                  <div class="user-item">
                    <div class="user" ondblclick={ondblclick.reform(move |_| user.clone())} >
                      <Avatar name={x.name.clone()} />
                      <span class="user-name">{x.name.clone()}</span>
                    </div>
                    <ul class="action">
                    { for options.iter().map(|y| {
                      let value = y.value.clone();
                      let user = x.clone();
                      html! {
                        <li class="icon">
                          if let Some(icon) = y.icon {
                            <Icon
                              onclick={onclick.reform(move |_| (user.clone(), value.clone()))}
                              icon_id={icon}
                              width="16px"
                              height="16px"
                            />
                          }
                        </li>
                      }
                    })}
                    </ul>
                  </div>
                }
              })}
            </div>
          </section>
        }
      })}
    </div>
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
      .user-group {
        margin-block: 10px;
      }
      .group-tag {
        text-transform: capitalize;
        font-size: 16px;
        margin-inline-start: 5px;
        color: var(--font-color);
      }
      .user-list {
        inline-size: 100%;
      }
      .user-item {
        inline-size: 100%;
        display: flex;
        align-items: center;
        margin-block: 20px;
        transition: background 0.2s ease;
        cursor: pointer;
        border-radius: var(--radius);
      }
      .user-item:hover {
        background: rgba(var(--theme-color-rgb), 0.5);
      }
      .user-item:hover .action { 
        transform: translate(0);
        inline-size: calc(var(--avatar-size, 40px) * 2);
      }
      .user-item:hover .user {
        inline-size: calc(100% - var(--avatar-size, 40px) * 2);
      }
      .user {
        display: flex;
        align-items: center;
        inline-size: 100%;
      }
      .user-name {
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
      .action {
        display: flex;
        align-items: center;
        transform: translate(calc(var(--avatar-size, 40px) * 2));
        transition: all 0.2s ease;
        inline-size: 0;
      }
      .icon {
        block-size: var(--avatar-size, 40px);
        inline-size: var(--avatar-size, 40px);
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--icon-button-background-hover);
        color: white;
        transition: all 0.2s ease;
      }
      .icon:last-of-type {
        border-inline-start: 1px solid rgba(var(--theme-color-rgb), 0.5);
      }
      .icon:hover {
        background: var(--icon-button-background-hover);
      }
    "#
  ))
}
