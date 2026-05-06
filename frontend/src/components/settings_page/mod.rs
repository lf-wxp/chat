//! Settings drawer (Task 23 / Req 13).
//!
//! The drawer is split across one file per visual section so each
//! component stays under the project-wide file-size guideline and can
//! be reasoned about in isolation. `SettingsPage` is the shell that
//! assembles them.

mod appearance_section;
mod av_section;
mod class_helpers;
mod data_management_helpers;
mod data_management_section;
mod device_select;
mod notifications_section;
mod page;
mod permission_badge;
mod privacy_section;

pub use page::SettingsPage;
