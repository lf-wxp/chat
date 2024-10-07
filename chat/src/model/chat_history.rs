use message::Information;
use std::collections::HashMap;

#[derive(PartialEq, Debug, Clone, Default)]
pub struct ChatHistory(pub HashMap<String, Vec<Information>>);
