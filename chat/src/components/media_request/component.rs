use stylist::{self, style};
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use super::MediaRequestContext;
use crate::{
  components::{
    use_media_confirm, use_media_reject, use_media_remove, Avatar, CallbackType, MediaMessage,
    MediaState,
  },
  utils::style,
};

#[function_component]
pub fn MediaRequest() -> Html {
  let class_name = get_class_name();
  let message_ctx = use_context::<MediaRequestContext>();
  let (message_list, callback_vec) =
    message_ctx.map_or((vec![], vec![]), |x| (x.list.clone(), x.callback.clone()));
  let media_remove = use_media_remove();
  let media_reject = use_media_reject();
  let media_confirm = use_media_confirm();

  let class_fn = |item: MediaMessage| {
    let mut extra = "";
    if item.state == MediaState::Perish {
      extra = "perish";
    }
    format!("media-message {extra}")
  };

  let remove_fn = Callback::from(move |message: MediaMessage| {
    if message.state == MediaState::Perish {
      media_remove(message.id);
    }
  });

  let callback_vec_clone = callback_vec.clone();
  let reject = Callback::from(move |message: MediaMessage| {
    media_reject(message.id.clone());
    callback_vec_clone
      .iter()
      .for_each(|f| f(message.clone(), CallbackType::Reject));
  });

  let confirm = Callback::from(move |message: MediaMessage| {
    media_confirm(message.id.clone());
    callback_vec
      .iter()
      .for_each(|f| f(message.clone(), CallbackType::Confirm));
  });

  html! {
    if message_list.iter().len() > 0 {
      <div class={class_name} >
        { for message_list.iter().map(|item|{
          let message = item.clone();
          let message_reject = item.clone();
          let message_confirm = item.clone();
          html!{
            <div
              key={item.id.clone()}
              class={class_fn(item.clone())}
              onanimationend={remove_fn.reform(move |_| message.clone())}
            >
              <div class="caller">
                <Avatar name={item.from_name.clone()} />
                <span class="name">{ item.from_name.clone()}</span>
              </div>
              <div class="action">
                <span class="confirm icon"
                  onclick={confirm.reform(move |_| message_confirm.clone())}
                >
                  <Icon
                    icon_id={IconId::LucidePhoneCall}
                    width="16px"
                    height="16px"
                  />
                </span>
                <span class="reject icon"
                  onclick={reject.reform(move |_| message_reject.clone())}
                >
                  <Icon
                    icon_id={IconId::LucidePhoneOff}
                    width="16px"
                    height="16px"
                  />
                </span>
              </div>
            </div>
        }})}
      </div>
    }
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        position: fixed;
        inset-block-start: 20px;
        inset-inline: 0;
        --avatar-size: 80px; 
        .media-message {
          padding: 20px;
          margin: auto;
          color: var(--font-color);
          background: rgba(var(--theme-color-rgb), 0.8);
          border-radius: var(--radius);
          backdrop-filter: blur(10px);
          text-align: center;
          margin-block-end: 10px;
          transition: all 0.2s ease;
          animation: fadeIn 0.2s;
          display: flex;
          align-items: center;
          inline-size: fit-content;
        }
        .caller {
          display: flex;
          flex-flow: column nowrap;
          align-items: flex-start;
          justify-content: space-between;
          margin-inline-end: 20px;
        }
        .name {
          margin-block-start: 5px;
        }
        .action {
          display: flex;
          flex-flow: column nowrap;
          height: 100%;
          justify-content: space-between;
        }
        .perish {
          animation: fadeOut 0.2s;
        }
        .icon {
          inline-size: 40px;
          block-size: 40px;
          border-radius: var(--radius);
          color: white;
          display: flex;
          align-items: center;
          justify-content: center;
          cursor: pointer;
        }
        avatar {
          animation: shake 0.5s;
          animation-iteration-count: infinite;
        }
        .reject {
          background: var(--danger-color);
          margin-block-start: 20px;
        }
        .confirm {
          background: var(--success-color);
        }

        @keyframes fadeIn {
          0% {
            opacity: 0;
            transform: translateY(-50%);
          } 
          100% {
            opacity: 1;
            transform: translateY(0);
          } 
        }
        @keyframes fadeOut {
          0% {
            opacity: 1;
            transform: translateY(0);
            margin-block-end: 10px;
          } 
          100% {
            opacity: 0;
            transform: translateY(-50%);
            margin-block-end: -28px;
          } 
        }
        @keyframes shake {
          0% { transform: translate(1px, 1px) rotate(0deg); }
          10% { transform: translate(-1px, -2px) rotate(-1deg); }
          20% { transform: translate(-3px, 0px) rotate(1deg); }
          30% { transform: translate(3px, 2px) rotate(0deg); }
          40% { transform: translate(1px, -1px) rotate(1deg); }
          50% { transform: translate(-1px, 2px) rotate(-1deg); }
          60% { transform: translate(-3px, 1px) rotate(0deg); }
          70% { transform: translate(3px, 1px) rotate(-1deg); }
          80% { transform: translate(-1px, -1px) rotate(1deg); }
          90% { transform: translate(1px, 2px) rotate(0deg); }
          100% { transform: translate(1px, -2px) rotate(-1deg); }
        }
    "#
  ))
}
