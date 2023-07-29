use chrono::prelude::*;
use stylist::{self, style};
use yew::prelude::*;

use crate::{
  components::Avatar,
  model::{Message, MessageAlignment},
  utils::style,
};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub name: Option<AttrValue>,
  pub uuid: Option<AttrValue>,
  pub alignment: MessageAlignment,
  pub time: DateTime<Utc>,
  pub message: Message,
}

#[function_component]
pub fn ChatMessage(props: &Props) -> Html {
  let class_name = get_class_name();
  let content_class = if props.alignment == MessageAlignment::Right {
    "current".to_string()
  } else {
    "".to_string()
  };

  html! {
    <div class={format!("{} {}", class_name, content_class)}>
      if let Some(name) = &props.name {
        <Avatar name={name} />
      }
      <div class={"message-content"}>
        <time>{{ props.time.format("%d/%m/ %H:%M") }}</time>
        if let Message::Text(text) = &props.message {
          <div class="message">{{ text }}</div>
        }
      </div>
    </div>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        display: flex;
        align-items: flex-start;
        --time-dot-bg: #97a8b9;
        avatar {
          flex: 0 0 auto;
        }
        time {
          color: var(--font-color);
          font-size: 12px;
          display: flex;
          align-items: center;
          white-space: nowrap;
          margin-block-end: 5px;
        }
        time::before {
          content: "";
          flex: 0 0 auto;
          background: var(--time-dot-bg);
          inline-size: 4px;
          block-size: 4px;
          border-radius: 50%;
          margin-inline-end: 5px;
        }
        .message {
          background: var(--theme-ancillary-color);
          border-radius: var(--radius);
          padding: 10px;
          color: var(--font-color);
          word-break: break-all;
        }
        .message-content {
          margin-inline: 5px;
          flex: 1 1 auto;
          display: flex;
          flex-flow: column nowrap;
          align-items: flex-start;
        }
        &.current {
          --theme-ancillary-color: #50b66d;
          --time-dot-bg: #50b66d;
          margin-inline-start: 45px; 
        }
        &.current .message {
          align-self: flex-end;    
        }
    "#
  ))
}
