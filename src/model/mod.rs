#[derive(PartialEq, Clone)]
pub struct Option<T = String> {
  pub label: String,
  pub value: T,
}

