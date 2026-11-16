//! One background thread: click-through (30 Hz), pause/battery/theme (1 Hz), and a
//! watchdog for monitor changes and Explorer restarts (every 3 s).

use crate::layers::{self, label};
use crate::platform::{self, Instance, Power};
use crate::state::AppState;
use crate::stats::Sampler;
use deskwave_core::config::KeepAwake;
use deskwave_core::manifest::Permission;
use std::collections::HashMap;
use std::sync::atomic::Ordering::SeqCst;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// While keeping awake, nobody may be watching all night: pause after this much idle time.
const AWAKE_IDLE_PAUSE_SECS: u64 = 10 * 60;

pub fn start(app: AppHandle, instance: Instance) {
    std::thread::spawn(move || {
        let mut ignoring = HashMap::new();
        let mut sampler = Sampler::default();
        let mut last_power = None;
        let mut last_accent = platform::accent_color();
        let mut tick: u64 = 0;
        loop {
            std::thread::sleep(Duration::from_millis(33));
            tick = tick.wrapping_add(1);
            click_through(&app, &mut ignoring);
            if tick.is_multiple_of(30) {
                system(&app, &mut last_power, &mut last_accent);
                if instance.take_show_request() {
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || layers::open_settings(&handle));
                }
            }
            if tick.is_multiple_of(60) {
                stats(&app, &mut sampler);
            }
            if tick.is_multiple_of(90) {
                watchdog(&app);
            }
        }
    });
}

/// A widget layer takes the mouse only while the cursor is over one of its widgets, so the
/// desktop icons underneath stay clickable.
fn click_through(app: &AppHandle, ignoring: &mut HashMap<String, bool>) {
    let state = app.state::<AppState>();
    let editing = state.editing.load(SeqCst);
    let (x, y) = platform::cursor_pos(app);
    let layers = state.hit_rects.lock().unwrap().clone();
    ignoring.retain(|label, _| layers.contains_key(label));
    for (label, rects) in layers {
        let over = editing
            || rects.iter().any(|r| x >= r[0] && y >= r[1] && x < r[0] + r[2] && y < r[1] + r[3]);
        if ignoring.get(&label) != Some(&!over) {
            if let Some(window) = app.get_webview_window(&label) {
                let _ = window.set_ignore_cursor_events(!over);
            }
            ignoring.insert(label, !over);
        }
    }
}

fn system(app: &AppHandle, last_power: &mut Option<Power>, last_accent: &mut String) {
    let state = app.state::<AppState>();
    let (options, awake) = {
        let config = state.config.lock().unwrap();
        (config.pause, config.keep_awake)
    };
    let monitors = state.monitors.lock().unwrap().clone();
    let power = platform::power();

    let coverage = if options.fullscreen || options.covered {
        platform::coverage(app, &monitors)
    } else {
        vec![Default::default(); monitors.len()]
    };
    let everywhere = state.user_paused.load(SeqCst)
        || (options.locked && platform::session_inactive())
        || (options.battery && (!power.on_ac || power.saver))
        || (awake != KeepAwake::Off && platform::idle_seconds() >= AWAKE_IDLE_PAUSE_SECS);
    let paused: Vec<bool> = coverage
        .iter()
        .map(|c| everywhere || (options.fullscreen && c.fullscreen) || (options.covered && c.maximized))
        .collect();

    let generation = state.generation.load(SeqCst);
    let mut previous = state.paused.lock().unwrap();
    for (i, &p) in paused.iter().enumerate() {
        if previous.get(i) != Some(&p) {
            let _ = app.emit_to(label("wallpaper", generation, i), "pause", p);
        }
    }
    *previous = paused;
    drop(previous);

    if *last_power != Some(power) {
        *last_power = Some(power);
        let _ = app.emit("battery", power);
    }
    let accent = platform::accent_color();
    if *last_accent != accent {
        let _ = app.emit("theme", &accent);
        *last_accent = accent;
    }
}

/// Every 2 s, only while a System widget is placed.
fn stats(app: &AppHandle, sampler: &mut Sampler) {
    let state = app.state::<AppState>();
    if !state.placed_with(Permission::System) {
        return;
    }
    let sample = sampler.sample();
    *state.stats.lock().unwrap() = sample;
    let _ = app.emit("system", sample);
}

fn watchdog(app: &AppHandle) {
    let state = app.state::<AppState>();
    let generation = state.generation.load(SeqCst);
    let count = state.monitors.lock().unwrap().len();
    let missing = (0..count).any(|i| {
        app.get_webview_window(&label("wallpaper", generation, i)).is_none()
            || app.get_webview_window(&label("widgets", generation, i)).is_none()
    });
    let owner_changed = platform::desktop_owner() != *state.owner.lock().unwrap();
    let monitors = layers::read_monitors(app);
    let monitors_changed = monitors != *state.monitors.lock().unwrap();
    if missing || owner_changed || monitors_changed {
        let app = app.clone();
        let _ = app.clone().run_on_main_thread(move || layers::rebuild(&app));
    }
}
