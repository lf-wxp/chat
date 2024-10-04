use std::{
  fmt::{self, Display},
  rc::Rc,
};
use bounce::Slice;
use yew::Reducible;

#[derive(PartialEq, Clone)]
pub enum RefreshAction {
  Toggle,
}
#[derive(Slice, PartialEq, Default)]
pub struct Refresh(bool);

impl Reducible for Refresh {
  type Action = RefreshAction;
  fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
    match action {
      RefreshAction::Toggle => {
        Self(!self.0).into()
      }
    }
  }
}

impl Display for Refresh {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}
