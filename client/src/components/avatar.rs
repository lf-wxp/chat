//! Avatar component

use leptos::prelude::*;
use leptos_i18n::t_string;

use crate::i18n::*;

/// Avatar size variants
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum AvatarSize {
  Small,
  #[default]
  Medium,
  Large,
}

/// Avatar component
#[component]
pub fn Avatar(
  /// Username (used to generate initials avatar)
  #[prop(into)]
  username: String,
  /// Avatar URL (optional)
  #[prop(into, optional)]
  src: Option<String>,
  /// Size variant
  #[prop(optional)]
  size: AvatarSize,
  /// Whether the user is online
  #[prop(optional)]
  online: bool,
) -> impl IntoView {
  let i18n = use_i18n();
  let size_class = match size {
    AvatarSize::Small => "avatar-sm",
    AvatarSize::Medium => "avatar-md",
    AvatarSize::Large => "avatar-lg",
  };

  let initials = username
    .chars()
    .next()
    .map(|c| c.to_uppercase().to_string())
    .unwrap_or_default();

  // Generate stable background color based on username
  let color_index = username
    .bytes()
    .fold(0u32, |acc, b| acc.wrapping_add(u32::from(b)))
    % 6;
  let bg_colors = [
    "#4f46e5", "#0891b2", "#059669", "#d97706", "#dc2626", "#7c3aed",
  ];
  let bg_color = bg_colors[color_index as usize];

  view! {
    <div class=format!("avatar {}", size_class) aria-label=format!("{}", t_string!(i18n, profile_avatar_of).replace("{}", &username))>
      {match src {
        Some(url) => view! { <img class="avatar-img" src=url alt=username.clone() /> }.into_any(),
        None => view! {
          <div class="avatar-initials" style=format!("background-color: {}", bg_color)>
            {initials}
          </div>
        }.into_any(),
      }}
      {if online {
        Some(view! {
          <span class="avatar-status online"></span>
        })
      } else {
        None
      }}
    </div>
  }
}
