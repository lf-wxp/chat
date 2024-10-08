use std::{env, io::Error as IoError, net::SocketAddr};
use futures::{
  future::{self, Either},
  pin_mut, StreamExt, TryStreamExt,
};
use message::RequestMessage;
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
  let client = Client::new(None, tx);
  let uuid_key = client.uuid();
  if let Some(client_map) = get_client_map() {
    client_map.insert(uuid_key.clone(), client);
  }
  let (sink, stream) = ws_stream.split();
  let (transform_tx, transform_rx) = unbounded_channel::<Message>();
  let message_tx = transform_tx.clone();
  let execute_message = stream.try_for_each(|msg| {
    let message = match bincode::deserialize::<RequestMessage>(&msg.into_data()) {
      Ok(message) => {
        println!("Received a message from {}: {:?}", addr, message,);
        message.execute(uuid_key.clone(), None)
      }
      Err(_) => None,
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
