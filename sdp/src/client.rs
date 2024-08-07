use nanoid::nanoid;
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::Message;

type Tx = UnboundedSender<Message>;

#[derive(Clone, Debug)]
pub struct Client {
  uuid: String,
  name: String,
  pub tx: Tx,
}

impl Client {
  pub fn new(name: Option<String>, tx: Tx) -> Client {
    let name = name.unwrap_or(nanoid!());
    let uuid = nanoid!();
    Client {
      uuid,
      name,
      tx,
    }
  }
  pub fn uuid(&self) -> String {
    self.uuid.clone()
  }

  pub fn update_name(&mut self, name: String) {
    self.name = name;
  }
}

impl From<&Client> for message::Client {
  fn from(client: &Client) -> Self {
    let Client { uuid, name, tx: _ } = client;
    message::Client {
      name: name.to_string(),
      uuid: uuid.to_string(),
    }
  }
}
