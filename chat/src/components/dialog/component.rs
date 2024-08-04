use stylist::{self, style};
use yew::prelude::*;

use super::{DialogContext, DialogItem};
use crate::{components::DialogAction, utils::style};

#[function_component]
pub fn DialogComponent() -> Html {
  let class_name = get_class_name();
  let dialog_context =
    use_context::<UseReducerHandle<DialogContext>>().expect("get empty dialog context");
  let class_content_fn = move |visible: bool| {
    let mut extra = "";
    if !visible {
      extra = "perish-container";
    }
    format!("dialog-container {class_name} {extra}")
  };
  let class_content = use_state(|| class_content_fn(false));
  let dialog_context_clone = dialog_context.clone();
  let container_hide = Callback::from(move |_: AnimationEvent| {
    if !dialog_context_clone.class_visible {
      dialog_context_clone.dispatch(DialogAction::Visible(false));
    }
  });

  let class_content_clone = class_content.clone();
  let dialog_context_clone = dialog_context.clone();
  use_effect_with(dialog_context_clone.class_visible, move |visible| {
    class_content_clone.set(class_content_fn(*visible));
  });

  let dialog_context_clone = dialog_context.clone();
  use_effect_with(dialog_context_clone.list.clone(), move |list| {
    let len = list.iter().len();
    let val = len != 0;
    if val {
      dialog_context_clone.dispatch(DialogAction::Visible(true));
    }
    dialog_context_clone.dispatch(DialogAction::ClassVisible(val));
  });
  html! {
    if dialog_context.visible {
      <div class={(*class_content).clone()} onanimationend={container_hide} >
        { for dialog_context.list.iter().map(|item|{
          html!{
            <DialogItem item={item.clone()} />
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
        &.perish-container {
          visibility: hidden;
          opacity: 0;
          animation: fadeOut 0.2s;
        }
        @keyframes fadeIn {
          0% {
            opacity: 0;
            visibility: hidden;
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
            visibility: hidden;
          } 
        }
    "#
  ))
}
