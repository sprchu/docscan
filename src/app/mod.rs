//! Application logic — state management, input handling, and UI regions.
//!
//! Each sub-module owns a focused slice of the `App` struct's behaviour:
//!
//! | Module | Responsibility |
//! |--------|---------------|
//! | [`types`] | Data types: `FileType`, `ScanResult`, `Focus`, `Mode`, … |
//! | [`state`] | `App` struct definition + constructor |
//! | [`config`] | Query editing, threads, file type toggles, dir selection |
//! | [`results`] | Result navigation, filter text/type, filtered view |
//! | [`browse`] | Directory browser enter/exit/navigate/confirm |
//! | [`cmd`] | Command-mode text editing |
//! | [`input`] | Keyboard event dispatch per focus/mode |
//! | [`mouse`] | Mouse event dispatch per UI region |

pub mod browse;
pub mod cmd;
pub mod config;
pub mod input;
pub mod mouse;
pub mod results;
pub mod state;
pub mod types;

// Re-export commonly used types for convenience
pub use state::App;
pub use types::{FileType, Focus, Mode, ScanResult, UiRects};
