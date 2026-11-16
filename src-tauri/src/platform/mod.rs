//! What differs between Windows and macOS: putting windows on the desktop, power and lock
//! state, accent color, languages, start at sign-in, keeping awake and opening links.
//! Both modules offer the same functions; the rest of the app calls them through here.

use serde::Serialize;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
pub use mac::*;
#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::*;

/// Passed by the sign-in entry so that start stays quiet; a normal launch opens Settings.
pub const BACKGROUND_FLAG: &str = "--background";

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Power {
    /// False on desktop computers.
    pub present: bool,
    pub percent: Option<u8>,
    pub charging: bool,
    pub on_ac: bool,
    /// Battery saver (Windows) or Low Power Mode (macOS).
    pub saver: bool,
    pub seconds_left: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Coverage {
    /// A window covers the whole display (a fullscreen game or video).
    pub fullscreen: bool,
    /// A maximized window is on the display.
    pub maximized: bool,
}
