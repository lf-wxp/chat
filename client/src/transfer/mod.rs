//! File Transfer Manager
//!
//! DataChannel-based P2P chunked transfer, supporting:
//! - Dynamic chunk size
//! - Resume from breakpoint (bitmap tracking)
//! - Transfer progress display
//! - Flow control mechanism

pub mod ui;

use std::collections::HashMap;

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use message::{
  envelope::{Envelope, Payload},
  transfer::{DEFAULT_CHUNK_SIZE, FileChunk, FileControl, TransferBitmap, TransferMeta},
};

use crate::{services::webrtc::PeerManager, state};

/// Transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
  /// Sending
  Sending,
  /// Receiving
  Receiving,
}

/// Single transfer task state
#[derive(Debug, Clone)]
pub struct TransferTask {
  /// Transfer metadata
  pub meta: TransferMeta,
  /// Transfer direction
  pub direction: TransferDirection,
  /// Peer user ID
  pub peer_id: String,
  /// Progress bitmap
  pub bitmap: Vec<u8>,
  /// Transferred bytes
  pub transferred_bytes: u64,
  /// Transfer status
  pub status: TransferStatus,
  /// Received data buffer (receiver side)
  pub received_chunks: HashMap<u32, Vec<u8>>,
}

/// Transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
  /// Waiting for peer confirmation
  Pending,
  /// Transferring
  Transferring,
  /// Paused
  Paused,
  /// Completed
  Completed,
  /// Cancelled
  Cancelled,
  /// Failed
  Failed,
}

/// File Transfer Manager
#[derive(Clone)]
pub struct TransferManager {
  /// All transfer tasks
  pub(crate) tasks: StoredValue<HashMap<String, TransferTask>>,
  /// Sender file data cache
  file_data_cache: StoredValue<HashMap<String, Vec<u8>>>,
}

impl TransferManager {
  /// Create and provide to context
  pub fn provide() {
    let manager = Self {
      tasks: StoredValue::new(HashMap::new()),
      file_data_cache: StoredValue::new(HashMap::new()),
    };
    provide_context(manager);
  }

  /// Get from context
  pub fn use_manager() -> Self {
    use_context::<Self>().expect("TransferManager not provided")
  }

  /// Initiate file transfer
  pub fn send_file(&self, peer_id: &str, file_name: String, file_data: Vec<u8>, mime_type: String) {
    let file_size = file_data.len() as u64;
    let meta = TransferMeta::new(file_name, file_size, mime_type, DEFAULT_CHUNK_SIZE);
    let transfer_id = meta.transfer_id.clone();

    // Cache file data
    self.file_data_cache.update_value(|cache| {
      cache.insert(transfer_id.clone(), file_data);
    });

    // Create transfer task
    let task = TransferTask {
      meta: meta.clone(),
      direction: TransferDirection::Sending,
      peer_id: peer_id.to_string(),
      bitmap: TransferBitmap::new(meta.total_chunks).to_bytes(),
      transferred_bytes: 0,
      status: TransferStatus::Pending,
      received_chunks: HashMap::new(),
    };

    self.tasks.update_value(|tasks| {
      tasks.insert(transfer_id.clone(), task);
    });

    // Send transfer request
    let user_state = state::use_user_state();
    let my_id = user_state.get_untracked().user_id.clone();
    let envelope = Envelope::new(
      my_id,
      vec![peer_id.to_string()],
      Payload::FileControl(FileControl::Request(meta)),
    );
    let manager = PeerManager::use_manager();
    let _ = manager.send_envelope(peer_id, &envelope);
  }

  /// Handle transfer control messages
  pub fn handle_file_control(&self, from: &str, control: FileControl) {
    match control {
      FileControl::Request(meta) => {
        let task = TransferTask {
          meta: meta.clone(),
          direction: TransferDirection::Receiving,
          peer_id: from.to_string(),
          bitmap: TransferBitmap::new(meta.total_chunks).to_bytes(),
          transferred_bytes: 0,
          status: TransferStatus::Pending,
          received_chunks: HashMap::new(),
        };
        self.tasks.update_value(|tasks| {
          tasks.insert(meta.transfer_id.clone(), task);
        });
        self.accept_transfer(&meta.transfer_id);
      }
      FileControl::Accept { transfer_id } => {
        self.tasks.update_value(|tasks| {
          if let Some(task) = tasks.get_mut(&transfer_id) {
            task.status = TransferStatus::Transferring;
          }
        });
        self.send_chunks(&transfer_id);
      }
      FileControl::Reject {
        transfer_id,
        reason,
      } => {
        web_sys::console::log_1(&format!("Transfer rejected: {transfer_id} - {reason:?}").into());
        self.tasks.update_value(|tasks| {
          if let Some(task) = tasks.get_mut(&transfer_id) {
            task.status = TransferStatus::Cancelled;
          }
        });
      }
      FileControl::Progress {
        transfer_id,
        received_bitmap,
      } => {
        self.tasks.update_value(|tasks| {
          if let Some(task) = tasks.get_mut(&transfer_id) {
            task.bitmap = received_bitmap;
          }
        });
      }
      FileControl::Complete { transfer_id } => {
        self.tasks.update_value(|tasks| {
          if let Some(task) = tasks.get_mut(&transfer_id) {
            task.status = TransferStatus::Completed;
          }
        });
        self.file_data_cache.update_value(|cache| {
          cache.remove(&transfer_id);
        });
      }
      FileControl::Cancel { transfer_id, .. } => {
        self.tasks.update_value(|tasks| {
          if let Some(task) = tasks.get_mut(&transfer_id) {
            task.status = TransferStatus::Cancelled;
          }
        });
      }
      FileControl::Resume {
        transfer_id,
        received_bitmap,
      } => {
        self.tasks.update_value(|tasks| {
          if let Some(task) = tasks.get_mut(&transfer_id) {
            task.bitmap = received_bitmap;
            task.status = TransferStatus::Transferring;
          }
        });
        self.send_chunks(&transfer_id);
      }
      FileControl::AdjustChunkSize {
        transfer_id,
        new_chunk_size,
      } => {
        self.tasks.update_value(|tasks| {
          if let Some(task) = tasks.get_mut(&transfer_id) {
            task.meta.chunk_size = new_chunk_size;
          }
        });
      }
    }
  }

  /// Handle received file chunks
  pub fn handle_file_chunk(&self, chunk: FileChunk) {
    let transfer_id = chunk.transfer_id.clone();
    let chunk_index = chunk.chunk_index;

    self.tasks.update_value(|tasks| {
      if let Some(task) = tasks.get_mut(&transfer_id) {
        let chunk_size = chunk.data.len() as u64;
        task.received_chunks.insert(chunk_index, chunk.data);
        task.transferred_bytes += chunk_size;

        let mut bitmap = TransferBitmap::from_bytes(task.bitmap.clone(), task.meta.total_chunks);
        bitmap.set(chunk_index);
        task.bitmap = bitmap.to_bytes();

        if bitmap.is_complete() {
          task.status = TransferStatus::Completed;
          let mut file_data = Vec::with_capacity(task.meta.file_size as usize);
          for i in 0..task.meta.total_chunks {
            if let Some(data) = task.received_chunks.get(&i) {
              file_data.extend_from_slice(data);
            }
          }
          trigger_download(&task.meta.file_name, &file_data, &task.meta.mime_type);

          let user_state = state::use_user_state();
          let my_id = user_state.get_untracked().user_id.clone();
          let envelope = Envelope::new(
            my_id,
            vec![task.peer_id.clone()],
            Payload::FileControl(FileControl::Complete {
              transfer_id: transfer_id.clone(),
            }),
          );
          let manager = PeerManager::use_manager();
          let _ = manager.send_envelope(&task.peer_id, &envelope);
        }
      }
    });
  }

  /// Accept transfer
  fn accept_transfer(&self, transfer_id: &str) {
    let peer_id = self.tasks.with_value(|tasks| {
      tasks
        .get(transfer_id)
        .map(|t| t.peer_id.clone())
        .unwrap_or_default()
    });

    self.tasks.update_value(|tasks| {
      if let Some(task) = tasks.get_mut(transfer_id) {
        task.status = TransferStatus::Transferring;
      }
    });

    let user_state = state::use_user_state();
    let my_id = user_state.get_untracked().user_id.clone();
    let envelope = Envelope::new(
      my_id,
      vec![peer_id.clone()],
      Payload::FileControl(FileControl::Accept {
        transfer_id: transfer_id.to_string(),
      }),
    );
    let manager = PeerManager::use_manager();
    let _ = manager.send_envelope(&peer_id, &envelope);
  }

  /// Send file chunks (with flow control)
  ///
  /// Serialize all pending chunks and hand them to FlowController for batch sending,
  /// avoiding flooding the DataChannel buffer at once.
  fn send_chunks(&self, transfer_id: &str) {
    let task_info = self.tasks.with_value(|tasks| {
      tasks
        .get(transfer_id)
        .map(|t| (t.peer_id.clone(), t.meta.clone(), t.bitmap.clone()))
    });

    let Some((peer_id, meta, bitmap_bytes)) = task_info else {
      return;
    };

    let file_data = self
      .file_data_cache
      .with_value(|cache| cache.get(transfer_id).cloned());

    let Some(file_data) = file_data else {
      web_sys::console::error_1(&"File data cache not found".into());
      return;
    };

    let bitmap = TransferBitmap::from_bytes(bitmap_bytes, meta.total_chunks);
    let user_state = state::use_user_state();
    let my_id = user_state.get_untracked().user_id.clone();

    // Pre-serialize all pending chunks
    let mut serialized_chunks: Vec<Vec<u8>> = Vec::new();
    for chunk_index in 0..meta.total_chunks {
      if bitmap.is_set(chunk_index) {
        continue;
      }

      let start = (chunk_index as usize) * meta.chunk_size;
      let end = ((chunk_index as usize + 1) * meta.chunk_size).min(file_data.len());
      let chunk_data = file_data[start..end].to_vec();

      let chunk = FileChunk {
        transfer_id: meta.transfer_id.clone(),
        chunk_index,
        data: chunk_data,
      };

      let envelope = Envelope::new(
        my_id.clone(),
        vec![peer_id.clone()],
        Payload::FileChunk(chunk),
      );

      match envelope.split(message::envelope::DEFAULT_CHUNK_THRESHOLD) {
        Ok(parts) => {
          for part in parts {
            serialized_chunks.push(part);
          }
        }
        Err(e) => {
          web_sys::console::error_1(
            &format!("Chunk {chunk_index} serialization failed: {e}").into(),
          );
        }
      }
    }

    if serialized_chunks.is_empty() {
      return;
    }

    let total = serialized_chunks.len();
    web_sys::console::log_1(
      &format!(
        "[File Transfer] transfer_id={}, pending={total} chunks, target={peer_id}",
        meta.transfer_id
      )
      .into(),
    );

    // Try to use flow control for sending
    let peer_manager = PeerManager::use_manager();
    if let Some(flow) = leptos::prelude::use_context::<crate::flow_control::FlowController>() {
      // Get DataChannel reference
      let dc = peer_manager.get_data_channel(&peer_id);
      if let Some(dc) = dc {
        flow.send_chunks_with_flow_control(peer_id, dc, serialized_chunks);
        return;
      }
    }

    // Fallback: without flow controller or DataChannel reference, send chunk by chunk via PeerManager
    for (i, chunk_bytes) in serialized_chunks.iter().enumerate() {
      // Send serialized data directly through underlying DataChannel
      if let Err(e) = peer_manager.send_raw(&peer_id, chunk_bytes) {
        web_sys::console::error_1(&format!("Failed to send chunk {i}/{total}: {e}").into());
        break;
      }
    }
  }

  /// Get transfer progress
  pub fn get_progress(&self, transfer_id: &str) -> Option<f64> {
    self.tasks.with_value(|tasks| {
      tasks.get(transfer_id).map(|task| {
        let bitmap = TransferBitmap::from_bytes(task.bitmap.clone(), task.meta.total_chunks);
        bitmap.progress_percent()
      })
    })
  }

  /// Get all active transfers
  pub fn active_transfers(&self) -> Vec<(String, TransferTask)> {
    self.tasks.with_value(|tasks| {
      tasks
        .iter()
        .filter(|(_, t)| {
          matches!(
            t.status,
            TransferStatus::Pending | TransferStatus::Transferring
          )
        })
        .map(|(id, t)| (id.clone(), t.clone()))
        .collect()
    })
  }
}

/// Trigger browser file download
fn trigger_download(file_name: &str, data: &[u8], mime_type: &str) {
  if let Some(window) = web_sys::window()
    && let Some(document) = window.document()
  {
    let array = js_sys::Uint8Array::new_with_length(data.len() as u32);
    array.copy_from(data);
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&array.buffer());

    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime_type);

    if let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options)
      && let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob)
      && let Ok(a) = document.create_element("a")
    {
      let a: web_sys::HtmlElement = a.unchecked_into();
      let _ = a.set_attribute("href", &url);
      let _ = a.set_attribute("download", file_name);
      a.set_hidden(true);
      if let Some(body) = document.body() {
        let _ = body.append_child(&a);
        a.click();
        let _ = body.remove_child(&a);
      }
      let _ = web_sys::Url::revoke_object_url(&url);
    }
  }
}
