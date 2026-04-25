//! Shared cursor iteration helpers for IndexedDB operations.

use crate::persistence::idb::{IdbResult, from_js};
use crate::persistence::record::MessageRecord;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::IdbCursorWithValue;

/// Iterate a cursor and deserialise up to `limit` records.
pub(super) async fn collect_messages_from_cursor(
  request: web_sys::IdbRequest,
  limit: usize,
) -> IdbResult<Vec<MessageRecord>> {
  if limit == 0 {
    return Ok(Vec::new());
  }
  let out = Rc::new(RefCell::new(Vec::<MessageRecord>::with_capacity(
    limit.min(64),
  )));
  let on_success: Rc<RefCell<Option<Closure<dyn FnMut(web_sys::Event)>>>> =
    Rc::new(RefCell::new(None));

  let promise = {
    let out = out.clone();
    let on_success_rc = on_success.clone();
    js_sys::Promise::new(&mut |resolve, reject| {
      let req_for_success = request.clone();
      let resolve_for_success = resolve.clone();
      let out_inner = out.clone();
      let on_success_inner = on_success_rc.clone();
      let cb: Closure<dyn FnMut(web_sys::Event)> = Closure::new(move |_: web_sys::Event| {
        let result = match req_for_success.result() {
          Ok(r) => r,
          Err(_) => {
            on_success_inner.borrow_mut().take();
            let _ = resolve_for_success.call0(&JsValue::UNDEFINED);
            return;
          }
        };
        if result.is_null() || result.is_undefined() {
          on_success_inner.borrow_mut().take();
          let _ = resolve_for_success.call0(&JsValue::UNDEFINED);
          return;
        }
        let Ok(cursor) = result.dyn_into::<IdbCursorWithValue>() else {
          on_success_inner.borrow_mut().take();
          let _ = resolve_for_success.call0(&JsValue::UNDEFINED);
          return;
        };
        if let Ok(value) = cursor.value()
          && let Ok(rec) = from_js::<MessageRecord>(&value)
        {
          out_inner.borrow_mut().push(rec);
        }
        if out_inner.borrow().len() >= limit {
          on_success_inner.borrow_mut().take();
          let _ = resolve_for_success.call0(&JsValue::UNDEFINED);
          return;
        }
        let _ = cursor.continue_();
      });
      request.set_onsuccess(Some(cb.as_ref().unchecked_ref()));
      *on_success_rc.borrow_mut() = Some(cb);

      let req_for_error = request.clone();
      let reject_clone = reject.clone();
      let on_error = Closure::once_into_js(move |_: web_sys::Event| {
        let err = req_for_error
          .error()
          .ok()
          .flatten()
          .map(JsValue::from)
          .unwrap_or_else(|| JsValue::from_str("IDB cursor error"));
        let _ = reject_clone.call1(&JsValue::UNDEFINED, &err);
      });
      request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    })
  };

  JsFuture::from(promise).await?;
  let collected = out.borrow().clone();
  Ok(collected)
}

/// Iterate a cursor and call `delete` on every visited entry. Returns
/// the number of records deleted.
pub(super) async fn iterate_cursor_delete(request: web_sys::IdbRequest) -> IdbResult<usize> {
  iterate_cursor_delete_limited(request, usize::MAX).await
}

/// As [`iterate_cursor_delete`], but stops after `limit` deletions.
pub(super) async fn iterate_cursor_delete_limited(
  request: web_sys::IdbRequest,
  limit: usize,
) -> IdbResult<usize> {
  use std::cell::Cell;

  if limit == 0 {
    return Ok(0);
  }

  let counter = Rc::new(Cell::new(0usize));
  let on_success: Rc<RefCell<Option<Closure<dyn FnMut(web_sys::Event)>>>> =
    Rc::new(RefCell::new(None));

  let promise = {
    let counter = counter.clone();
    let on_success_rc = on_success.clone();
    js_sys::Promise::new(&mut |resolve, reject| {
      let req_for_success = request.clone();
      let resolve_clone = resolve.clone();
      let counter_inner = counter.clone();
      let on_success_inner = on_success_rc.clone();
      let cb: Closure<dyn FnMut(web_sys::Event)> = Closure::new(move |_: web_sys::Event| {
        let result = match req_for_success.result() {
          Ok(r) => r,
          Err(_) => {
            on_success_inner.borrow_mut().take();
            let _ = resolve_clone.call0(&JsValue::UNDEFINED);
            return;
          }
        };
        if result.is_null() || result.is_undefined() {
          on_success_inner.borrow_mut().take();
          let _ = resolve_clone.call0(&JsValue::UNDEFINED);
          return;
        }
        let Ok(cursor) = result.dyn_into::<IdbCursorWithValue>() else {
          on_success_inner.borrow_mut().take();
          let _ = resolve_clone.call0(&JsValue::UNDEFINED);
          return;
        };
        let _ = cursor.delete();
        counter_inner.set(counter_inner.get().saturating_add(1));
        if counter_inner.get() >= limit {
          on_success_inner.borrow_mut().take();
          let _ = resolve_clone.call0(&JsValue::UNDEFINED);
          return;
        }
        let _ = cursor.continue_();
      });
      request.set_onsuccess(Some(cb.as_ref().unchecked_ref()));
      *on_success_rc.borrow_mut() = Some(cb);

      let req_for_error = request.clone();
      let reject_clone = reject.clone();
      let on_error = Closure::once_into_js(move |_: web_sys::Event| {
        let err = req_for_error
          .error()
          .ok()
          .flatten()
          .map(JsValue::from)
          .unwrap_or_else(|| JsValue::from_str("IDB cursor delete error"));
        let _ = reject_clone.call1(&JsValue::UNDEFINED, &err);
      });
      request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    })
  };
  JsFuture::from(promise).await?;
  Ok(counter.get())
}

/// Iterate a cursor, deleting entries where `matches` returns true.
/// Returns the number of deletions.
pub(super) async fn iterate_cursor_delete_matching(
  request: web_sys::IdbRequest,
  matches: impl Fn(&JsValue) -> bool + 'static,
) -> IdbResult<usize> {
  use std::cell::Cell;

  let counter = Rc::new(Cell::new(0usize));
  let on_success: Rc<RefCell<Option<Closure<dyn FnMut(web_sys::Event)>>>> =
    Rc::new(RefCell::new(None));
  let matches = Rc::new(RefCell::new(matches));

  let promise = {
    let counter = counter.clone();
    let on_success_rc = on_success.clone();
    let matches_rc = matches.clone();
    js_sys::Promise::new(&mut |resolve, reject| {
      let req_for_success = request.clone();
      let resolve_clone = resolve.clone();
      let counter_inner = counter.clone();
      let on_success_inner = on_success_rc.clone();
      let matches_inner = matches_rc.clone();
      let cb: Closure<dyn FnMut(web_sys::Event)> = Closure::new(move |_: web_sys::Event| {
        let result = match req_for_success.result() {
          Ok(r) => r,
          Err(_) => {
            on_success_inner.borrow_mut().take();
            let _ = resolve_clone.call0(&JsValue::UNDEFINED);
            return;
          }
        };
        if result.is_null() || result.is_undefined() {
          on_success_inner.borrow_mut().take();
          let _ = resolve_clone.call0(&JsValue::UNDEFINED);
          return;
        }
        let Ok(cursor) = result.dyn_into::<IdbCursorWithValue>() else {
          on_success_inner.borrow_mut().take();
          let _ = resolve_clone.call0(&JsValue::UNDEFINED);
          return;
        };
        if let Ok(value) = cursor.value() {
          if (matches_inner.borrow_mut())(&value) {
            let _ = cursor.delete();
            counter_inner.set(counter_inner.get().saturating_add(1));
          }
        }
        let _ = cursor.continue_();
      });
      request.set_onsuccess(Some(cb.as_ref().unchecked_ref()));
      *on_success_rc.borrow_mut() = Some(cb);

      let req_for_error = request.clone();
      let reject_clone = reject.clone();
      let on_error = Closure::once_into_js(move |_: web_sys::Event| {
        let err = req_for_error
          .error()
          .ok()
          .flatten()
          .map(JsValue::from)
          .unwrap_or_else(|| JsValue::from_str("IDB cursor delete matching error"));
        let _ = reject_clone.call1(&JsValue::UNDEFINED, &err);
      });
      request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    })
  };
  JsFuture::from(promise).await?;
  Ok(counter.get())
}
