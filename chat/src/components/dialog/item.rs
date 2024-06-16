use stylist::{self, style};
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use super::{use_pre_remove_dialog, use_remove_dialog, Dialog, DialogState};
use crate::utils::style;

#[derive(Properties, PartialEq)]
pub struct Props {
  pub item: Dialog,
  #[prop_or_default]
  pub children: Html,
  #[prop_or_default]
  pub onclose: Callback<()>,
  #[prop_or_default]
  pub onpreclose: Callback<()>,
}

#[function_component]
pub fn DialogItem(props: &Props) -> Html {
  let class_name = get_class_name();
  let dialog = props.item.clone();
  let item = props.item.clone();
  let onclose = props.onclose.clone();
  let on_pre_close = props.onpreclose.clone();
  let remove = use_remove_dialog();
  let pre_remove = use_pre_remove_dialog();

  let class_fn = |item: Dialog| {
    let mut extra = "";
    if item.state == DialogState::Perish {
      extra = "perish";
    }
    format!("dialog {class_name} {extra}")
  };

  let remove_fn = Callback::from(move |(e, dialog): (AnimationEvent, Dialog)| {
    e.stop_propagation();
    if dialog.state == DialogState::Perish {
      onclose.emit(());
      remove(dialog.id);
    }
  });

  let pre_remove_fn = Callback::from(move |id: String| {
    on_pre_close.emit(());
    pre_remove(id);
  });

  html! {
      <section
        key={dialog.id.clone()}
        class={class_fn(dialog.clone())}
        onanimationend={remove_fn.reform(move |e| (e, item.clone()))}
      >
        <header>
          <p class="header">{dialog.header.clone()}</p>
          <Icon
            icon_id={IconId::BootstrapX}
            class="icon"
            width="20px"
            height="20px"
            onclick={pre_remove_fn.reform(move |_| dialog.id.clone())}
          />
        </header>
        <div class="content">
          {props.children.clone()}
          {dialog.content.clone()}
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
        position: absolute;
        inset-block: 0;
        inset-inline: 0;
        margin: auto;
        padding: 20px;
        inline-size: fit-content;
        block-size: fit-content;
        max-inline-size: 50vw;
        max-block-size: 50vh;
        min-inline-size: 300px;
        color: var(--font-color);
        background: rgba(var(--theme-color-rgb), 0.8);
        border-radius: var(--radius);
        backdrop-filter: blur(10px);
        transition: all 0.2s ease;
        animation: fadePedIn 0.2s;
        header {
          display:flex;
          align-items: center;
        }
        .header {
          flex: 1 1 auto;
          font-size: 20px;
          font-weight: bolder;
          padding: 10px;
          margin: 0;
        }
        .content {
          padding: 10px;
        }
        &.perish {
          animation: fadePedOut 0.2s;
        }
        .icon {
          margin-inline-start: 5px;
          transition: all 0.2s ease;
          cursor: pointer;
        }
        .icon:hover {
          background: rgba(var(--theme-color-rgb), 1);
        }

        @keyframes fadePedIn {
          0% {
            opacity: 0;
            transform: translateY(-50%);
          } 
          100% {
            opacity: 1;
            transform: translateY(0);
          } 
        }
        @keyframes fadePedOut {
          0% {
            opacity: 1;
            transform: translateY(0);
          } 
          100% {
            opacity: 0;
            transform: translateY(50%);
          } 
        }
    "#
  ))
}
