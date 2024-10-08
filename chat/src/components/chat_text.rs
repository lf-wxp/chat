use stylist::{self, style};
use web_sys::HtmlTextAreaElement;
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::utils::{get_target, get_textarea_selection_offset, style};

#[derive(Clone)]
pub struct Selection {
  pub start: Option<u32>,
  pub end: Option<u32>,
}

#[derive(Clone)]
pub struct ChatValue {
  pub value: String,
  pub selection: Selection,
}

#[derive(Properties, PartialEq)]
pub struct Props {
  pub text: String,
  pub onchange: Callback<ChatValue>,
  pub onsend: Callback<()>,
}

#[function_component]
pub fn ChatText(props: &Props) -> Html {
  let class_name = get_class_name();
  let textarea_ref = use_node_ref();

  let resize_textarea = {
    let textarea_ref = textarea_ref.clone();
    move || {
      let textarea = textarea_ref
        .cast::<HtmlTextAreaElement>()
        .expect("div_ref not attached to div element");
      textarea.set_attribute("style", "height: auto").unwrap();
      let scroll_height = textarea.scroll_height();
      textarea
        .set_attribute("style", &format!("height: {}px", scroll_height)[..])
        .unwrap();
    }
  };

  let onsend = {
    let send = props.onsend.clone();
    Callback::from(move |_| {
      send.emit(());
    })
  };

  let onblur = {
    let change = props.onchange.clone();
    Callback::from(move |e: FocusEvent| {
      let target = get_target::<FocusEvent, HtmlTextAreaElement>(e);
      if let Some(target) = target {
        let value = target.value();
        change.emit(ChatValue {
          value: value.clone(),
          selection: get_textarea_selection_offset(target, &value),
        });
      }
    })
  };

  let onresize = {
    let resize = resize_textarea.clone();
    Callback::from(move |_: InputEvent| {
      resize();
    })
  };

  let text = props.text.clone();
  let resize = resize_textarea;
  use_effect_with(text, move |_| resize());

  html! {
    <div class={class_name}>
      <textarea
        ref={textarea_ref}
        value={props.text.clone()}
        class="textarea scroll-bar"
        onblur={onblur}
        oninput={onresize}
        rows="1"
      />
      <span class="send-btn" onclick={onsend}>
        <Icon icon_id={IconId::BootstrapSendFill}  width="16px" height="16px" />
      </span>
    </div>
  }
}

#[allow(non_upper_case_globals)]
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
        background: var(--icon-button-background);
        color: white; 
        inline-size: var(--send-size);
        block-size: var(--send-size);
        flex: 0 0 auto;
        cursor: pointer;
        border-radius: calc(var(--radius) / 2);
        transition: all 0.2s ease;
      } 
      .send-btn:hover {
        background: var(--icon-button-background-hover);
      }
         
    "#
  ))
}
