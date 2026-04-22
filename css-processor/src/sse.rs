//! Minimal Server-Sent Events (SSE) broadcaster for CSS hot-reload.
//!
//! Implements a tiny HTTP/1.1 server using only `std::net`, exposing a single
//! endpoint `GET /events` that streams `text/event-stream` frames. Each call to
//! [`Broadcaster::notify`] pushes a `rebuild` event to every connected client.
//!
//! Design goals:
//! - **Zero polling**: browsers establish one long-lived connection and simply
//!   wait for events. No network traffic occurs when files do not change.
//! - **Zero new dependencies**: avoids pulling in a full HTTP crate for what
//!   amounts to ~100 lines of protocol work.
//! - **Best-effort delivery**: dropped clients are silently pruned on the next
//!   broadcast; no reconnection bookkeeping on the server side (the browser's
//!   built-in EventSource auto-reconnect handles that).

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

/// Shared handle to broadcast rebuild events to all connected browsers.
#[derive(Clone)]
pub struct Broadcaster {
  clients: Arc<Mutex<Vec<TcpStream>>>,
}

impl Broadcaster {
  /// Start an SSE server bound to `127.0.0.1:<port>` and return a broadcaster handle.
  ///
  /// The listener runs on a background thread; each accepted connection
  /// serving `GET /events` is kept alive and registered for future broadcasts.
  /// All other requests receive a short 404 response.
  pub fn start(port: u16) -> Result<Self> {
    let addr = format!("127.0.0.1:{port}");
    let listener =
      TcpListener::bind(&addr).with_context(|| format!("Failed to bind SSE server on {addr}"))?;
    eprintln!("[css-processor] CSS hot-reload SSE server listening on http://{addr}/events");

    let clients: Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));
    let clients_for_thread = Arc::clone(&clients);

    thread::spawn(move || {
      for conn in listener.incoming() {
        match conn {
          Ok(stream) => {
            let clients = Arc::clone(&clients_for_thread);
            thread::spawn(move || handle_connection(stream, clients));
          }
          Err(e) => eprintln!("[css-processor] SSE accept error: {e}"),
        }
      }
    });

    Ok(Self { clients })
  }

  /// Broadcast a `rebuild` event with an optional JSON payload to all clients.
  ///
  /// Disconnected clients (write errors) are pruned from the roster.
  pub fn notify(&self, payload: &str) {
    let frame = format!("event: rebuild\ndata: {payload}\n\n");
    let mut guard = match self.clients.lock() {
      Ok(g) => g,
      Err(poisoned) => poisoned.into_inner(),
    };

    let mut alive = Vec::with_capacity(guard.len());
    for mut stream in guard.drain(..) {
      if stream.write_all(frame.as_bytes()).is_ok() && stream.flush().is_ok() {
        alive.push(stream);
      }
    }
    let remaining = alive.len();
    *guard = alive;
    if remaining > 0 {
      eprintln!("[css-processor] SSE: broadcasted rebuild to {remaining} client(s)");
    }
  }
}

/// Handle a single HTTP connection: serve `GET /events` as SSE, otherwise 404.
fn handle_connection(mut stream: TcpStream, clients: Arc<Mutex<Vec<TcpStream>>>) {
  // Read request line + headers
  let request_line = {
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
      return;
    }
    // Drain headers so client isn't stuck waiting for us to read.
    loop {
      let mut hdr = String::new();
      match reader.read_line(&mut hdr) {
        Ok(0) => break,
        Ok(_) if hdr == "\r\n" || hdr == "\n" => break,
        Ok(_) => continue,
        Err(_) => return,
      }
    }
    line
  };

  let is_events = request_line.starts_with("GET /events");

  if !is_events {
    let _ = stream.write_all(
      b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
    );
    return;
  }

  // Send SSE headers. `Access-Control-Allow-Origin: *` lets the Trunk-served
  // page (a different origin/port) connect without CORS preflight concerns.
  let headers = "HTTP/1.1 200 OK\r\n\
    Content-Type: text/event-stream\r\n\
    Cache-Control: no-cache\r\n\
    Connection: keep-alive\r\n\
    Access-Control-Allow-Origin: *\r\n\
    X-Accel-Buffering: no\r\n\
    \r\n\
    retry: 2000\n\n";

  if stream.write_all(headers.as_bytes()).is_err() || stream.flush().is_err() {
    return;
  }

  // Disable read timeout so the socket stays open; set a long write timeout
  // to avoid hanging on dead peers during broadcast.
  let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

  // Hand the stream over to the broadcaster registry.
  match clients.lock() {
    Ok(mut guard) => guard.push(stream),
    Err(poisoned) => poisoned.into_inner().push(stream),
  }
}
