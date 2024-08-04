use nanoid::nanoid;
use yew::prelude::*;

use super::{Dialog, DialogAction, DialogContext, DialogItem, DialogState};
use crate::utils::query_selector;

#[derive(Properties, PartialEq)]
pub struct Props {
  #[prop_or_default]
  pub visible: bool,
  #[prop_or_default]
  pub content: String,
  #[prop_or_default]
  pub header: String,
  #[prop_or_default]
  pub children: Html,
  #[prop_or_default]
  pub onclose: Callback<()>,
}

#[function_component]
pub fn DialogPortal(props: &Props) -> Html {
  let host = query_selector(".dialog-container");
  let onclose = props.onclose.clone();
  let item = use_state(|| Dialog {
    id: nanoid!(),
    header: props.header.clone(),
    content: props.content.clone(),
    state: DialogState::Exist,
  });
  let dialog_context =
    use_context::<UseReducerHandle<DialogContext>>().expect("get empty dialog context");

  use_effect_with(props.visible, move |visible| {
    if *visible {
      dialog_context.dispatch(DialogAction::ClassVisible(true));
      dialog_context.dispatch(DialogAction::Visible(true));
      return;
    }
    dialog_context.dispatch(DialogAction::ClassVisible(false));
  });

  if !props.visible || host.is_none() {
    return html! {};
  }

  let item_clone = item.clone();
  let onpreclose = Callback::from(move |_| {
    item_clone.set(Dialog {
      state: DialogState::Perish,
      ..(*item_clone).clone()
    });
  });

  let dom = html! {
    <DialogItem item={(*item).clone()} {onclose} {onpreclose}>
     { props.children.clone() }
    </DialogItem>
  };

  create_portal(dom, host.expect(""))
}
