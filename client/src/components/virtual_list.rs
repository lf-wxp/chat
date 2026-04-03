//! Virtual scrolling list component
//!
//! Renders only the visible items in a scrollable container,
//! significantly improving performance for large lists (>100 items).

use leptos::prelude::*;

/// Number of extra items to render above and below the visible area.
const OVERSCAN: usize = 5;

/// A generic virtual scrolling list component.
///
/// Only renders the items that are currently visible in the viewport,
/// plus a small overscan buffer for smooth scrolling.
///
/// # Props
/// - `items`: The full list of items (reactive signal).
/// - `item_height`: Fixed height of each item in pixels.
/// - `container_height`: Height of the scrollable container in pixels.
/// - `render_item`: A closure that renders a single item given `(index, item)`.
/// - `class`: Optional CSS class for the outer container.
#[component]
pub fn VirtualList<T, F, V>(
  /// The full list of items.
  items: Signal<Vec<T>>,
  /// Fixed height of each item in pixels.
  #[prop(default = 48.0)]
  item_height: f64,
  /// Height of the scrollable container in pixels.
  /// If 0, the container will try to fill its parent.
  #[prop(default = 0.0)]
  container_height: f64,
  /// Render function: (index, item) -> View
  render_item: F,
  /// Optional CSS class for the outer container.
  #[prop(optional, into)]
  class: String,
) -> impl IntoView
where
  T: Clone + Send + Sync + 'static,
  F: Fn(usize, T) -> V + Clone + Send + Sync + 'static,
  V: IntoView + 'static,
{
  let scroll_top = RwSignal::new(0.0_f64);
  let container_ref = NodeRef::<leptos::html::Div>::new();

  // Compute the actual container height (use prop or measure from DOM)
  let actual_height = Memo::new(move |_| {
    if container_height > 0.0 {
      return container_height;
    }
    // Fallback: try to read from the DOM element
    if let Some(el) = container_ref.get() {
      let rect = el.get_bounding_client_rect();
      let h = rect.height();
      if h > 0.0 {
        return h;
      }
    }
    400.0 // default fallback
  });

  // Compute visible range
  let visible_range = Memo::new(move |_| {
    let total = items.get().len();
    if total == 0 {
      return (0, 0);
    }
    let h = actual_height.get();
    let top = scroll_top.get();
    let start = ((top / item_height).floor() as usize).saturating_sub(OVERSCAN);
    let visible_count = (h / item_height).ceil() as usize + 2 * OVERSCAN;
    let end = (start + visible_count).min(total);
    (start, end)
  });

  // Total height of all items (for the spacer)
  let total_height = Memo::new(move |_| items.get().len() as f64 * item_height);

  // Handle scroll events
  let on_scroll = move |_ev: web_sys::Event| {
    if let Some(el) = container_ref.get() {
      let el: &web_sys::Element = &el;
      scroll_top.set(el.scroll_top() as f64);
    }
  };

  let container_style = move || {
    if container_height > 0.0 {
      format!("overflow-y: auto; position: relative; height: {container_height}px;")
    } else {
      "overflow-y: auto; position: relative; height: 100%;".to_string()
    }
  };

  view! {
    <div
      node_ref=container_ref
      class=format!("virtual-list {class}")
      style=container_style
      on:scroll=on_scroll
    >
      // Spacer to maintain total scroll height
      <div style=move || format!("height: {}px; position: relative;", total_height.get())>
        // Render only visible items
        {move || {
          let (start, end) = visible_range.get();
          let all_items = items.get();
          let render = render_item.clone();

          (start..end)
            .map(|i| {
              let item = all_items[i].clone();
              let offset_y = i as f64 * item_height;
              let style = format!(
                "position: absolute; top: {offset_y}px; left: 0; right: 0; height: {item_height}px;"
              );
              view! {
                <div style=style>
                  {render(i, item)}
                </div>
              }
            })
            .collect::<Vec<_>>()
        }}
      </div>
    </div>
  }
}
