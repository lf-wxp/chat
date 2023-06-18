use std::{
  fmt::{self, Display},
  rc::Rc,
};
use yew::{html, virtual_dom::Key, Html};
use yew_router::prelude::*;

use crate::page::{Home, NotFound, Setting, User, Video};

#[derive(Clone, Routable, PartialEq, Debug)]
pub enum Route {
  #[at("/")]
  Home,
  #[at("/user")]
  User,
  #[at("/video")]
  Video,
  #[at("/setting")]
  Setting,
  #[not_found]
  #[at("/404")]
  NotFound,
}

impl Into<Key> for Route {
  fn into(self) -> Key {
    match self {
      Route::Home => Key::from(Rc::<str>::from("home")),
      Route::User => Key::from(Rc::<str>::from("user")),
      Route::Video => Key::from(Rc::<str>::from("video")),
      Route::Setting => Key::from(Rc::<str>::from("setting")),
      Route::NotFound => Key::from(Rc::<str>::from("notFound")),
    }
  }
}

impl Display for Route {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Route::Home => write!(f, "home"),
      Route::User => write!(f, "user"),
      Route::Video => write!(f, "video"),
      Route::Setting => write!(f, "setting"),
      Route::NotFound => write!(f, "notFound"),
    }
  }
}

pub fn switch(routes: Route) -> Html {
  match routes {
    Route::Home => html! { <Home /> },
    Route::User => html! { <User /> },
    Route::Video => html! { <Video /> },
    Route::Setting => html! { <Setting /> },
    Route::NotFound => html! { <NotFound />},
  }
}
