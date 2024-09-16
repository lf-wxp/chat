use stylist::{self, style};
use yew::prelude::*;

use crate::{components::{UserList, Search, VideoStream}, utils::style};

#[function_component]
pub fn User() -> Html {
  let class_name = get_class_name();

  html! {
    <section class={class_name}>
      <div class="user-box">
        <Search />
        <div class="user-list-container scroll-bar">
          <UserList />
        </div>
      </div>
      <div class="user-video">
        <VideoStream />
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
    .user-box {
      inline-size: 300px;
      block-size: 100%;
      flex: 0 0 auto;
    }
    .user-list-container {
      block-size: calc(100% - 32px);
      overflow-y: auto;
      overflow-x: hidden;
    }
    .user-video {

    }
    "#
  ))
}
