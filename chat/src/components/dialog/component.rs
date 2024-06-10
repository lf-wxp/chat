use stylist::{self, style};
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use super::{use_pre_remove_dialog, use_remove_dialog, Dialog, DialogContext, DialogState};
use crate::utils::style;

#[function_component]
pub fn DialogComponent() -> Html {
  let class_name = get_class_name();
  let dialog_list = use_context::<DialogContext>().map_or(vec![], |x| x.0.clone());
  let remove = use_remove_dialog();
  let pre_remove = use_pre_remove_dialog();
  let visible = use_state(|| false);
  let class_visible = use_state(|| false);

  let class_fn = |item: Dialog| {
    let mut extra = "";
    if item.state == DialogState::Perish {
      extra = "perish";
    }
    format!("dialog {extra}")
  };

  let class_content_fn = || {
    let mut extra = "";
    if !*class_visible {
      extra = "perish-container";
    }
    format!("{class_name} {extra}")
  };

  let remove_fn = Callback::from(move |dialog: Dialog| {
    if dialog.state == DialogState::Perish {
      remove(dialog.id);
    }
  });

  let pre_remove_fn = Callback::from(move |id: String| {
    pre_remove(id);
  });

  let visible_clone = visible.clone();
  let class_visible_clone = class_visible.clone();
  let container_hide = Callback::from(move |_: AnimationEvent| {
    if !*class_visible_clone {
      visible_clone.set(false);
    }
  });

  let dialog_list_clone = dialog_list.clone();
  let visible_clone = visible.clone();
  let class_visible_clone = class_visible.clone();
  use_effect_with(dialog_list_clone.iter().len(), move |len: &usize| {
    let val = *len != 0;
    if val {
      visible_clone.set(true);
    }
    class_visible_clone.set(val);
  });

  html! {
    if *visible {
      <div class={class_content_fn()} onanimationend={container_hide} >
        { for dialog_list.iter().map(|item|{
          let dialog = item.clone();
          let id = item.id.clone();
          html!{
            <section
              key={item.id.clone()}
              class={class_fn(item.clone())}
              onanimationend={remove_fn.reform(move |_| dialog.clone())}
            >
              <header>
                <p class="header">{item.header.clone()}</p>
                <Icon
                  icon_id={IconId::BootstrapX}
                  class="icon"
                  width="20px"
                  height="20px"
                  onclick={pre_remove_fn.reform(move |_| id.clone())}
                />
              </header>
              <div class="content">
                {item.content.clone()}
              </div>
            </section>
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
        inset-block: 0;
        inset-inline: 0;
        inline-size: 100vw;
        block-size: 100vh; 
        background: rgba(var(--theme-color-rgb), 0.2);
        transition: all 0.2s ease;
        animation: fadeIn 0.2s;
        opacity: 1;
        z-index: 10;
        .dialog {
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
        }
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
        .perish {
          animation: fadePedOut 0.2s;
        }
        &.perish-container {
          visibility: visible;
          opacity: 0;
          animation: fadeOut 0.2s;
        }
        .icon {
          margin-inline-start: 5px;
          transition: all 0.2s ease;
          cursor: point;
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
        @keyframes fadeIn {
          0% {
            opacity: 0;
            visibility: hide;
          } 
          100% {
            opacity: 1;
            visibility: visible;
          } 
        }
        @keyframes fadeOut {
          0% {
            opacity: 1;
            visibility: visible;
          } 
          100% {
            opacity: 0;
            visibility: hide;
          } 
        }
    "#
  ))
}
