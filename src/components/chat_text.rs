use stylist::{self, style};
use web_sys::HtmlTextAreaElement;
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::utils::{get_target, style};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub text: String,
  pub onchange: Callback<String>,
  pub onsend: Callback<()>,
}

#[function_component]
pub fn ChatText(props: &Props) -> Html {
  let class_name = get_class_name();

  let onsend = {
    let send = props.onsend.clone();
    Callback::from(move |_| {
      send.emit(());
    })
  };

  let onchange = {
    let change = props.onchange.clone();
    Callback::from(move |e: FocusEvent| {
      let target = get_target::<FocusEvent, HtmlTextAreaElement>(e);
      if let Some(target) = target {
        let value = target.value();
        change.emit(value);
      }
    })
  };

  let onresize = Callback::from(move |e: InputEvent| {
    let target = get_target::<InputEvent, HtmlTextAreaElement>(e);
    if let Some(target) = target {
      target.set_attribute("style", "height: auto").unwrap();
      let scroll_height = target.scroll_height();
      target
        .set_attribute("style", &format!("height: {}px", scroll_height)[..])
        .unwrap();
    }
  });

  html! {
    <div class={class_name}>
      <textarea
        value={props.text.clone()}
        class="textarea scroll-bar"
        onblur={onchange}
        oninput={onresize}
        rows="1"
      />
      <span class="send-btn" onclick={onsend}>
        <Icon icon_id={IconId::BootstrapSendFill}  width="16px" height="16px" />
      </span>
    </div>
  }
}

fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      --padding: 5px;
      --send-size: 40px;
      padding: var(--padding);
      background: var(--theme-ancillary-color);
      border-radius: calc(var(--radius) / 2);
      display: flex;
      align-items: flex-end;
      .textarea {
        flex:  1 1 auto;
        border: none;
        background: none;
        inline-size: calc(100% - var(--send-size));
        color: var(--font-color);
        line-height: 20px;
        min-block-size: 20px;
        max-block-size: 200px;
        overflow-y: auto;
        padding: calc(var(--padding) * 2);
        resize: none;
      }
      .textarea:active, .textarea:focus-visible {
        border: none;
        outline: none;
      }
      .send-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        background: #50b66d;
        color: white; 
        inline-size: var(--send-size);
        block-size: var(--send-size);
        flex: 0 0 auto;
        cursor: pointer;
        border-radius: calc(var(--radius) / 2);
        transition: all 0.2s ease;
      } 
      .send-btn:hover {
        background: #4cad68;
      }
         
    "#
  ))
}
