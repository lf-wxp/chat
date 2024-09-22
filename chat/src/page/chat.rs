use stylist::{self, style};
use yew::prelude::*;

use crate::{components::{ChatList, Search}, utils::style};

#[function_component]
pub fn Chat() -> Html {
  let class_name = get_class_name();
  let keyword = use_state(|| "".to_string());
  let onsearch = {
    let keyword_clone = keyword.clone();
    Callback::from(move |val: String| {
      keyword_clone.set(val);
    })
  };
  html! {
    <section class={class_name}>
      <div class="chat-box">
        <Search {onsearch} />
        <div class="chat-list-container scroll-bar">
          <ChatList keyword={(*keyword).clone()} />
        </div>
      </div>
    </section>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
    display: flex;
    
    block-size: 100%;
    inline-size: 100%;
    .chat-box {
      inline-size: 300px;
      block-size: 100%;
      flex: 0 0 auto;
    }
    .chat-list-container {
      block-size: calc(100% - 32px);
      overflow-y: auto;
      overflow-x: hidden;
    }
    "#
  ))
}
