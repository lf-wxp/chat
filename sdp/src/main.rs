use std::{env, io::Error as IoError, net::SocketAddr, cell::OnceCell};

use futures::{
  future::{self, Either},
  pin_mut, StreamExt, TryStreamExt,
};
use message::{ActionMessage, ListResponse, RequestMessage, State};
use sender_sink::wrappers::UnboundedSenderSink;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::unbounded_channel;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{
  accept_hdr_async,
  tungstenite::{
    handshake::server::{Request, Response},
    protocol::Message,
  },
};

use crate::action::BroadcastExecute;

static mut CLIENT_ID: OnceCell<i8> = OnceCell::new();

pub fn get_client_id() -> i8 {
  unsafe {
    CLIENT_ID.get_or_init(|| 1);
    if let Some(id) = CLIENT_ID.get_mut() {
      *id += 1;
      return *id;     
    }
    0
  }
}

mod action;
mod client;
mod data;

use {
  action::{msg_try_into, ParamResponseOptionExecute},
  client::Client,
  data::get_client_map,
};

async fn handle_connection(raw_stream: TcpStream, addr: SocketAddr) {
  println!("Incoming TCP connection from: {}", addr);

  let get_headers = |req: &Request, response: Response| {
    println!("Received a new ws handshake");
    println!("The request's path is: {}", req.uri().path());
    println!("The request's headers are:");
    for (ref header, _value) in req.headers() {
      println!("* {}: {:?}", header, _value);
    }
    Ok(response)
  };

  let ws_stream = accept_hdr_async(raw_stream, get_headers)
    .await
    .expect("Error during the websocket handshake occurred");
  println!("WebSocket connection established: {}", addr);

  let (tx, rx) = unbounded_channel();
  let client = Client::new(addr, None, tx);
  let uuid_key = client.uuid();

  if let Some(client_map) = get_client_map() {
    client_map.insert(uuid_key.clone(), client);
  }

  ListResponse {}.execute();

  let (sink, stream) = ws_stream.split();

  let (transform_tx, transform_rx) = unbounded_channel::<Message>();

  let message_tx = transform_tx.clone();

  let execute_message = stream.try_for_each(|msg| {
    let message = match serde_json::from_str::<RequestMessage>(msg.to_text().unwrap()) {
      Ok(message) => {
        println!(
          "Received a message from {}: {}",
          addr,
          msg.to_text().unwrap()
        );
        message.execute(uuid_key.clone())
      }
      Err(_) => Some(ActionMessage::to_resp_msg(
        State::Error,
        "construct".to_owned(),
        None,
      )),
    };

    if let Some(message) = message {
      message_tx.send(msg_try_into(message).unwrap()).unwrap();
    }

    future::ok(())
  });

  let transform_task = UnboundedReceiverStream::new(transform_rx)
    .map(Ok)
    .forward(sink);

  let receive_tx = transform_tx.clone();
  let receive_from_others = UnboundedReceiverStream::new(rx)
    .map(Ok)
    .forward(UnboundedSenderSink::from(receive_tx));

  pin_mut!(execute_message, receive_from_others, transform_task);
  match future::select(
    future::select(execute_message, receive_from_others),
    transform_task,
  )
  .await
  {
    Either::Left((value, _)) => match value {
      Either::Left((value1, _)) => println!("broadcast {:?}", value1),
      Either::Right((value2, _)) => println!("receive {:?}", value2),
    },
    Either::Right((value2, _)) => println!("receive {:?}", value2),
  }

  println!("{} disconnected", &addr);
  if let Some(client_map) = get_client_map() {
    client_map.remove(&uuid_key);
  }
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
  let addr = env::args()
    .nth(1)
    .unwrap_or_else(|| "127.0.0.1:8888".to_string());

  let try_socket = TcpListener::bind(&addr).await;
  let listener = try_socket.expect("Failed to bind");
  println!("Listening on: {}", addr);

  while let Ok((stream, addr)) = listener.accept().await {
    tokio::spawn(handle_connection(stream, addr));
  }

  Ok(())
}
