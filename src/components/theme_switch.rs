use bounce::use_atom;
use stylist::{self, style};
use yew::{prelude::*, Callback};
use yew_icons::{Icon, IconId};

use crate::{store::Theme, utils::style};

#[function_component]
pub fn ThemeSwitch() -> Html {
  let class_name = get_class_name();
  let theme = use_atom::<Theme>();

  let set_light = {
    let theme_clone = theme.clone();
    Callback::from(move |_: MouseEvent| {
      theme_clone.set(Theme::Light);
    })
  };

  let set_dark = {
    let theme_clone = theme.clone();
    Callback::from(move |_: MouseEvent| {
      theme_clone.set(Theme::Dark);
    })
  };

  let icon_class = |inner_theme: Theme| {
    let active = if *theme == inner_theme { "active" } else { "" };
    format!("switch-icon {inner_theme} {active}")
  };

  html! {
    <section class={class_name}>
      <div class={format!("mask {}", *theme)} />
      <span class={icon_class(Theme::Light)} onclick={set_light}>
        <Icon  icon_id={IconId::BootstrapSunFill} width="16px" height="16px" />
      </span>
      <span class={icon_class(Theme::Dark)} onclick={set_dark}>
        <Icon  icon_id={IconId::BootstrapMoonStarsFill} width="14px" height="14px" />
      </span>
    </section>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        --gap: 2px;
        --size: 32px;
        background: var(--theme-color);
        border-radius: var(--radius);
        cursor: pointer;
        block-size: var(--size);
        inline-size: calc(var(--size) * 2);
        transition: all .2s ease;
        align-items: center;
        display: flex;
        padding: var(--gap);
        position: relative;
        .switch-icon {
          inline-size: 50%;
          block-size: 100%;
          flex: 1 1 auto;
          align-items: center;
          justify-content: center;
          display: flex;
          border-radius: var(--radius);
          color: #B0B0B0;
          position: relative;
          transition: inherit;
        }
        .switch-icon:hover {
          color: #5F5F5F;
        }
        .light.active {
          color: #FFBD2D;
        }
        .dark.active {
          color: white;
        }
        .mask {
          inline-size: calc(50% - var(--gap));
          block-size: calc(100% - var(--gap) * 2);
          inset-inline-start: var(--gap);
          inset-block-start: var(--gap);
          position: absolute;
          border-radius: var(--radius);
          background: var(--theme-ancillary-color);
          transition: inherit;
        }
        .mask.dark {
          inset-inline-start: 50%;
        }
        svg {
          flex: 0 0 auto;
        }
    "#
  ))
}
