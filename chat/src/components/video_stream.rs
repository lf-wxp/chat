use bounce::use_atom_value;
use stylist::{self, style};
use yew::prelude::*;

use crate::{
  components::{use_register_callback, CallbackType},
  store::User,
  utils::{get_client_execute, style},
};

#[function_component]
pub fn VideoStream() -> Html {
  let class_name = get_class_name();
  let user = use_atom_value::<User>();
  use_register_callback(|message, callback_type| {
    let id = message.id.clone();
    match callback_type {
      CallbackType::Confirm => {
        get_client_execute(Box::new(|client| {
          Box::pin(async move {
            client.confirm_request_media(message.into(), id).await;
          })
        }));
      }
      CallbackType::Reject => {
        get_client_execute(Box::new(|client| {
          Box::pin(async move {
            client.reject_request_media(message.into(), id).await;
          })
        }));
      }
    };
  });

  html! {
    <>
      <div class={class_name}>
        {user.uuid.clone()}
        <video class="local-stream" autoplay={true} />
        <video class="remote-stream" autoplay={true} />
      </div>
    </>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      background: var(--theme-ancillary-color);
      border-radius: var(--radius);
      overflow: hidden;
      position: relative;
      video {
        border-radius: var(--radius);
        aspect-ratio: 3 / 2;
        object-fit: cover;
      }
      .local-stream {
        inline-size: 100%;
        block-size: 100%;
      }
      .remote-stream {
        position: absolute; 
        inset-inline-end: 0;
        inset-block-start: 0;
        inline-size: 40%;
      }
    "#
  ))
}
