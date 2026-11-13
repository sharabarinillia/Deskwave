//! Now playing, from the Windows media session (the same source as the volume flyout).
//! WinRT calls run on one dedicated thread, which also polls for changes.

use crate::state::AppState;
use deskwave_core::manifest::Permission;
use serde::Serialize;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct NowPlaying {
    /// False when nothing is playing or paused anywhere.
    pub active: bool,
    pub playing: bool,
    pub title: String,
    pub artist: String,
    pub app: String,
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Toggle,
    Next,
    Previous,
}

pub fn start(app: AppHandle) -> Sender<Action> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || run(app, rx));
    tx
}

fn run(app: AppHandle, actions: Receiver<Action>) {
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let manager = SessionManager::RequestAsync().and_then(|op| op.join()).ok();
    loop {
        match actions.recv_timeout(Duration::from_secs(1)) {
            Ok(action) => {
                if let Some(manager) = &manager {
                    let _ = act(manager, action);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let state = app.state::<AppState>();
        if !state.placed_with(Permission::Media) {
            continue;
        }
        let now = manager.as_ref().and_then(|m| read(m).ok()).unwrap_or_default();
        let mut last = state.media.lock().unwrap();
        if *last != now {
            *last = now.clone();
            drop(last);
            let _ = app.emit("media", now);
        }
    }
}

fn read(manager: &SessionManager) -> windows::core::Result<NowPlaying> {
    let session = manager.GetCurrentSession()?;
    let props = session.TryGetMediaPropertiesAsync()?.join()?;
    let status = session.GetPlaybackInfo()?.PlaybackStatus()?;
    Ok(NowPlaying {
        active: true,
        playing: status == PlaybackStatus::Playing,
        title: props.Title()?.to_string(),
        artist: props.Artist()?.to_string(),
        app: friendly_app(&session.SourceAppUserModelId()?.to_string()),
    })
}

fn act(manager: &SessionManager, action: Action) -> windows::core::Result<bool> {
    let session = manager.GetCurrentSession()?;
    match action {
        Action::Toggle => session.TryTogglePlayPauseAsync()?.join(),
        Action::Next => session.TrySkipNextAsync()?.join(),
        Action::Previous => session.TrySkipPreviousAsync()?.join(),
    }
}

/// `Spotify.exe` → `Spotify`; `Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic` → `ZuneMusic`.
fn friendly_app(id: &str) -> String {
    let base = id.split('!').next().unwrap_or(id).split('_').next().unwrap_or(id);
    let base = base.strip_suffix(".exe").or_else(|| base.strip_suffix(".EXE")).unwrap_or(base);
    base.rsplit('.').next().unwrap_or(base).to_owned()
}
