use stylist::{self, style};
use yew::prelude::*;
use yew_icons::IconId;

use crate::{
  components::{NavItem, VolumeSet, Account},
  route::Route,
  utils::style,
};

#[derive(Debug, Clone)]
struct Nav {
  icon: IconId,
  route: Route,
}

#[function_component]
pub fn Side() -> Html {
  let class_name = get_class_name();

  let nav_items = [
    Nav {
      icon: IconId::HeroiconsMiniSolidHome,
      route: Route::Home,
    },
    Nav {
      icon: IconId::HeroiconsSolidUserGroup,
      route: Route::User,
    },
    Nav {
      icon: IconId::HeroiconsSolidChatBubbleLeft,
      route: Route::Chat,
    },
    Nav {
      icon: IconId::BootstrapCameraVideoFill,
      route: Route::Video,
    },
    Nav {
      icon: IconId::LucideSettings,
      route: Route::Setting,
    },
  ];

  html! {
    <side class={class_name}>
      <div class="side-nav">
        { for nav_items.iter().map(|item|{
          html!{
            <NavItem key={item.route.clone()} route={item.route.clone()} icon={item.icon} />
        }})}
      </div>
      <div class="volume-set">
        <VolumeSet />
      </div>
      <div class="user-set">
        <Account />
      </div>
    </side>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        inline-size: 100%;
        block-size: 100%;
        display: flex;
        flex-flow: column nowrap;
        justify-content: center;
        align-items: center;

        .side-nav {
          display: flex;
          position: relative;
          flex-flow: column nowrap;
          justify-content: center;
          align-items: center;
        }
        .volume-set {
          position: absolute;
          inset-block-end: 45px; 
        }
        .user-set {
          position: absolute;
          inset-block-end: 5px; 
        }
    "#
  ))
}
