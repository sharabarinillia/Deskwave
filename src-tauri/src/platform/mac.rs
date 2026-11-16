//! macOS: desktop-level windows, power and lock state, accent color, languages, start at
//! sign-in (a LaunchAgent) and keeping the Mac awake (an IOKit power assertion).
//! AppKit is reached through `msg_send!` on the windows Tauri creates, always on the main thread.

use super::{Coverage, Power, BACKGROUND_FLAG};
use crate::layers::label;
use crate::state::{AppState, Monitor};
use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation_sys::base::{CFGetTypeID, CFRelease, CFTypeRef};
use core_foundation_sys::dictionary::{CFDictionaryGetValue, CFDictionaryRef};
use core_foundation_sys::locale::CFLocaleCopyPreferredLanguages;
use core_foundation_sys::number::{
    kCFNumberSInt64Type, CFBooleanGetTypeID, CFBooleanGetValue, CFNumberGetTypeID, CFNumberGetValue,
};
use core_foundation_sys::preferences::{kCFPreferencesAnyApplication, CFPreferencesCopyAppValue};
use core_foundation_sys::string::CFStringGetTypeID;
use deskwave_core::config::KeepAwake;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send, sel};
use std::ffi::c_void;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

pub const PLATFORM: &str = "macos";
/// macOS offers apps no public way to read or control what other apps are playing.
pub const MEDIA: bool = false;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowLevelForKey(key: i32) -> i32;
    fn CGSessionCopyCurrentDictionary() -> CFDictionaryRef;
    fn CGEventSourceSecondsSinceLastEventType(source_state: i32, event_type: u32) -> f64;
}

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOPMAssertionCreateWithName(kind: CFStringRef, level: u32, name: CFStringRef, id: *mut u32) -> i32;
    fn IOPMAssertionRelease(id: u32) -> i32;
    fn IOPSCopyPowerSourcesInfo() -> CFTypeRef;
    fn IOPSCopyPowerSourcesList(blob: CFTypeRef) -> CFArrayRef;
    fn IOPSGetPowerSourceDescription(blob: CFTypeRef, source: CFTypeRef) -> CFDictionaryRef;
}

// CGWindowLevelKey values.
const DESKTOP_LEVEL: i32 = 2;
const DESKTOP_ICON_LEVEL: i32 = 18;
/// NSFloatingWindowLevel: above normal windows, used while editing widgets.
const FLOATING_LEVEL: isize = 3;
/// NSWindowCollectionBehavior: on every Space, left alone by Mission Control, out of Cmd-`.
const BEHAVIOR: usize = 1 | (1 << 4) | (1 << 6);
/// NSWindowOcclusionStateVisible.
const OCCLUSION_VISIBLE: usize = 1 << 1;

fn level(key: i32) -> isize {
    unsafe { CGWindowLevelForKey(key) as isize }
}

/// Just above the desktop icons, below every app window.
fn widget_level() -> isize {
    level(DESKTOP_ICON_LEVEL) + 1
}

/// Runs `f` on the main thread with the window's NSWindow, if it still exists.
fn with_ns_window(window: &WebviewWindow, f: impl FnOnce(&AnyObject) + Send + 'static) {
    let target = window.clone();
    let _ = window.run_on_main_thread(move || {
        if let Ok(ptr) = target.ns_window() {
            if !ptr.is_null() {
                f(unsafe { &*ptr.cast::<AnyObject>() });
            }
        }
    });
}

// ---- windows on the desktop ----

/// No desktop window owns ours on macOS; window levels keep the layers in place.
pub fn desktop_owner() -> Option<isize> {
    None
}

/// Wallpaper: at the desktop picture's level, under the icons, on every Space, click-through.
pub fn place_wallpaper(window: &WebviewWindow, m: &Monitor) -> Result<(), String> {
    window.set_position(PhysicalPosition::new(m.rect.0, m.rect.1)).map_err(|e| e.to_string())?;
    window.set_size(PhysicalSize::new(m.rect.2, m.rect.3)).map_err(|e| e.to_string())?;
    with_ns_window(window, |w| unsafe {
        let _: () = msg_send![w, setLevel: level(DESKTOP_LEVEL)];
        let _: () = msg_send![w, setCollectionBehavior: BEHAVIOR];
        let _: () = msg_send![w, setIgnoresMouseEvents: true];
        let _: () = msg_send![w, setHasShadow: false];
        let _: () = msg_send![w, orderFrontRegardless];
    });
    Ok(())
}

/// Widget layer: just above the desktop icons and below apps, on every Space.
pub fn place_widgets(window: &WebviewWindow, _owner: Option<isize>) -> tauri::Result<()> {
    with_ns_window(window, |w| unsafe {
        let _: () = msg_send![w, setLevel: widget_level()];
        let _: () = msg_send![w, setCollectionBehavior: BEHAVIOR];
        let _: () = msg_send![w, setHasShadow: false];
        let _: () = msg_send![w, orderFrontRegardless];
    });
    Ok(())
}

/// Keeps a widget layer at its level after it is clicked.
pub fn sink(window: &WebviewWindow, _owner: Option<isize>) {
    with_ns_window(window, |w| unsafe {
        let _: () = msg_send![w, setLevel: widget_level()];
    });
}

/// Edit mode lifts a widget layer above all windows; leaving it puts it back.
pub fn set_editing(window: &WebviewWindow, on: bool, _owner: Option<isize>) {
    let target = if on { FLOATING_LEVEL } else { widget_level() };
    with_ns_window(window, move |w| unsafe {
        let _: () = msg_send![w, setLevel: target];
    });
}

/// Nothing to restore: once our windows close, the desktop picture shows again.
pub fn repaint_static_wallpaper() {}

pub fn cursor_pos(app: &AppHandle) -> (f64, f64) {
    app.cursor_position().map_or((f64::MIN, f64::MIN), |p| (p.x, p.y))
}

/// A wallpaper counts as covered when macOS reports its window as fully hidden: by windows,
/// a full-screen app or another Space. Both pause options map to this.
pub fn coverage(app: &AppHandle, monitors: &[Monitor]) -> Vec<Coverage> {
    let count = monitors.len();
    let generation = app.state::<AppState>().generation.load(SeqCst);
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let hidden: Vec<bool> = (0..count)
            .map(|i| {
                let window = handle.get_webview_window(&label("wallpaper", generation, i));
                window.and_then(|w| w.ns_window().ok()).filter(|p| !p.is_null()).is_some_and(|p| {
                    let w = unsafe { &*p.cast::<AnyObject>() };
                    let state: usize = unsafe { msg_send![w, occlusionState] };
                    state & OCCLUSION_VISIBLE == 0
                })
            })
            .collect();
        let _ = tx.send(hidden);
    });
    let hidden = rx.recv_timeout(Duration::from_millis(500)).unwrap_or_default();
    (0..count)
        .map(|i| {
            let h = hidden.get(i).copied().unwrap_or(false);
            Coverage { fullscreen: h, maximized: h }
        })
        .collect()
}

// ---- web view hardening ----

/// Defence in depth for every web view, like WebView2's browser arguments on Windows. Mods are
/// kept off the network by their CSP and sandbox; this covers the case of a gap in those:
/// - macOS 14+: WebKit's networking goes through a proxy on a closed local port, so a request
///   that slipped through would have nowhere to go;
/// - WebRTC is turned off, so no page can open peer-to-peer connections.
///
/// Both use API newer than macOS 12 or not public, so they are looked up at run time and
/// skipped where missing, rather than linked (which would stop older Macs from starting the app).
pub fn harden_webview(window: &WebviewWindow) {
    static PROXY: std::sync::Once = std::sync::Once::new();
    PROXY.call_once(|| unsafe { dead_proxy() });
    let _ = window.with_webview(|webview| unsafe {
        let Some(view) = webview.inner().cast::<AnyObject>().as_ref() else {
            return;
        };
        let config: *mut AnyObject = msg_send![view, configuration];
        let Some(config) = config.as_ref() else {
            return;
        };
        let prefs: *mut AnyObject = msg_send![config, preferences];
        let Some(prefs) = prefs.as_ref() else {
            return;
        };
        let known: bool = msg_send![prefs, respondsToSelector: sel!(_setPeerConnectionEnabled:)];
        if known {
            let _: () = msg_send![prefs, _setPeerConnectionEnabled: false];
        }
    });
}

/// Points the default website data store, which all of Deskwave's web views share, at a proxy
/// on 127.0.0.1:9, where nothing listens.
unsafe fn dead_proxy() {
    type CreateEndpoint = unsafe extern "C" fn(*const libc::c_char, *const libc::c_char) -> *mut AnyObject;
    type CreateProxy = unsafe extern "C" fn(*mut AnyObject, *mut AnyObject) -> *mut AnyObject;
    libc::dlopen(c"/System/Library/Frameworks/Network.framework/Network".as_ptr(), libc::RTLD_LAZY);
    let endpoint_fn = libc::dlsym(libc::RTLD_DEFAULT, c"nw_endpoint_create_host".as_ptr());
    let proxy_fn = libc::dlsym(libc::RTLD_DEFAULT, c"nw_proxy_config_create_http_connect".as_ptr());
    if endpoint_fn.is_null() || proxy_fn.is_null() {
        return; // before macOS 14
    }
    let store: *mut AnyObject = msg_send![class!(WKWebsiteDataStore), defaultDataStore];
    let Some(store) = store.as_ref() else {
        return;
    };
    let supported: bool = msg_send![store, respondsToSelector: sel!(setProxyConfigurations:)];
    if !supported {
        return;
    }
    let create_endpoint = std::mem::transmute::<*mut c_void, CreateEndpoint>(endpoint_fn);
    let create_proxy = std::mem::transmute::<*mut c_void, CreateProxy>(proxy_fn);
    let endpoint = create_endpoint(c"127.0.0.1".as_ptr(), c"9".as_ptr());
    if endpoint.is_null() {
        return;
    }
    let proxy = create_proxy(endpoint, std::ptr::null_mut());
    if proxy.is_null() {
        return;
    }
    let list: *mut AnyObject = msg_send![class!(NSArray), arrayWithObject: proxy];
    let _: () = msg_send![store, setProxyConfigurations: list];
}

// ---- Core Foundation helpers ----

unsafe fn dict_get(dict: CFDictionaryRef, key: &str) -> CFTypeRef {
    let key = CFString::new(key);
    CFDictionaryGetValue(dict, key.as_concrete_TypeRef() as *const c_void)
}

unsafe fn cf_i64(value: CFTypeRef) -> Option<i64> {
    if value.is_null() || CFGetTypeID(value) != CFNumberGetTypeID() {
        return None;
    }
    let mut out = 0i64;
    CFNumberGetValue(value as _, kCFNumberSInt64Type, &mut out as *mut i64 as *mut c_void).then_some(out)
}

unsafe fn cf_bool(value: CFTypeRef) -> Option<bool> {
    (!value.is_null() && CFGetTypeID(value) == CFBooleanGetTypeID()).then(|| CFBooleanGetValue(value as _))
}

unsafe fn cf_string(value: CFTypeRef) -> Option<String> {
    (!value.is_null() && CFGetTypeID(value) == CFStringGetTypeID())
        .then(|| CFString::wrap_under_get_rule(value as CFStringRef).to_string())
}

// ---- system state ----

/// The internal battery, if there is one, from IOKit's power sources.
pub fn power() -> Power {
    let mut p = Power {
        present: false,
        percent: None,
        charging: false,
        on_ac: true,
        saver: low_power_mode(),
        seconds_left: None,
    };
    unsafe {
        let blob = IOPSCopyPowerSourcesInfo();
        if blob.is_null() {
            return p;
        }
        let list = IOPSCopyPowerSourcesList(blob);
        if !list.is_null() {
            for i in 0..CFArrayGetCount(list) {
                let desc = IOPSGetPowerSourceDescription(blob, CFArrayGetValueAtIndex(list, i));
                if desc.is_null() || cf_string(dict_get(desc, "Type")).as_deref() != Some("InternalBattery") {
                    continue;
                }
                p.present = true;
                let current = cf_i64(dict_get(desc, "Current Capacity"));
                let max = cf_i64(dict_get(desc, "Max Capacity"));
                if let (Some(current), Some(max)) = (current, max) {
                    if max > 0 {
                        p.percent = Some((current * 100 / max).clamp(0, 100) as u8);
                    }
                }
                p.charging = cf_bool(dict_get(desc, "Is Charging")).unwrap_or(false);
                p.on_ac = cf_string(dict_get(desc, "Power Source State")).is_none_or(|s| s == "AC Power");
                let minutes = cf_i64(dict_get(desc, "Time to Empty")).filter(|m| *m > 0 && !p.on_ac);
                p.seconds_left = minutes.map(|m| (m * 60) as u32);
            }
            CFRelease(list as CFTypeRef);
        }
        CFRelease(blob);
    }
    p
}

fn low_power_mode() -> bool {
    unsafe {
        let info: *mut AnyObject = msg_send![class!(NSProcessInfo), processInfo];
        let Some(info) = info.as_ref() else {
            return false;
        };
        let known: bool = msg_send![info, respondsToSelector: sel!(isLowPowerModeEnabled)];
        known && msg_send![info, isLowPowerModeEnabled]
    }
}

/// The screen is locked, or another user's session is in front.
pub fn session_inactive() -> bool {
    unsafe {
        let dict = CGSessionCopyCurrentDictionary();
        if dict.is_null() {
            return false;
        }
        let locked = cf_bool(dict_get(dict, "CGSSessionScreenIsLocked")).unwrap_or(false);
        let on_console = cf_bool(dict_get(dict, "kCGSSessionOnConsoleKey")).unwrap_or(true);
        CFRelease(dict as CFTypeRef);
        locked || !on_console
    }
}

/// Seconds since the last keyboard, mouse or trackpad input.
pub fn idle_seconds() -> u64 {
    // kCGEventSourceStateCombinedSessionState, kCGAnyInputEventType.
    unsafe { CGEventSourceSecondsSinceLastEventType(0, u32::MAX).max(0.0) as u64 }
}

/// The accent color chosen in System Settings → Appearance, as `#rrggbb`.
pub fn accent_color() -> String {
    let index = unsafe {
        let key = CFString::from_static_string("AppleAccentColor");
        let value = CFPreferencesCopyAppValue(key.as_concrete_TypeRef(), kCFPreferencesAnyApplication);
        let index = cf_i64(value).or_else(|| cf_string(value).and_then(|s| s.parse().ok()));
        if !value.is_null() {
            CFRelease(value);
        }
        index
    };
    match index {
        Some(-1) => "#8c8c8c", // graphite
        Some(0) => "#e0383e",  // red
        Some(1) => "#f7821b",  // orange
        Some(2) => "#ffc600",  // yellow
        Some(3) => "#62ba46",  // green
        Some(5) => "#953d96",  // purple
        Some(6) => "#f74f9e",  // pink
        _ => "#007aff",        // blue, also "multicolor"
    }
    .into()
}

/// Preferred languages, e.g. `["uk-UA", "en-US"]`.
pub fn ui_languages() -> Vec<String> {
    unsafe {
        let list = CFLocaleCopyPreferredLanguages();
        if list.is_null() {
            return Vec::new();
        }
        let out = (0..CFArrayGetCount(list)).filter_map(|i| cf_string(CFArrayGetValueAtIndex(list, i))).collect();
        CFRelease(list as CFTypeRef);
        out
    }
}

// ---- keeping awake ----

static ASSERTION: Mutex<Option<u32>> = Mutex::new(None);

/// Holds an IOKit power assertion while keeping awake is on. macOS lists it under
/// `pmset -g assertions`, and it ends with the app.
pub fn set_keep_awake(mode: KeepAwake) -> Result<(), String> {
    let mut held = ASSERTION.lock().unwrap();
    if let Some(id) = held.take() {
        unsafe {
            IOPMAssertionRelease(id);
        }
    }
    let kind = match mode {
        KeepAwake::Off => return Ok(()),
        KeepAwake::Screen => "PreventUserIdleDisplaySleep",
        KeepAwake::System => "PreventUserIdleSystemSleep",
    };
    let (kind, name) = (CFString::new(kind), CFString::from_static_string("Deskwave: Keep awake is on"));
    let mut id = 0u32;
    // 255 is kIOPMAssertionLevelOn.
    let result = unsafe { IOPMAssertionCreateWithName(kind.as_concrete_TypeRef(), 255, name.as_concrete_TypeRef(), &mut id) };
    if result != 0 {
        return Err(format!("macOS did not allow keeping awake (error {result})"));
    }
    *held = Some(id);
    Ok(())
}

// ---- launching ----

/// Holds a lock file for as long as Deskwave runs.
pub struct Instance(#[allow(dead_code)] Option<std::fs::File>);

/// `None` if Deskwave is already running. Opening the app from Finder, Launchpad or the Dock
/// while it runs doesn't start a second copy: macOS sends `Reopen` to this one instead.
pub fn single_instance() -> Option<Instance> {
    let path = std::env::temp_dir().join("app.deskwave.desktop.lock"); // per user on macOS
    let Ok(file) = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(path) else {
        return Some(Instance(None));
    };
    let locked = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0;
    locked.then(|| Instance(Some(file)))
}

impl Instance {
    pub fn take_show_request(&self) -> bool {
        false
    }
}

const AGENT: &str = "app.deskwave.desktop";

fn agent_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/LaunchAgents").join(format!("{AGENT}.plist")))
}

pub fn autostart_enabled() -> bool {
    agent_path().is_some_and(|p| p.is_file())
}

/// Adds or removes a LaunchAgent that starts Deskwave quietly when the user signs in.
pub fn set_autostart(on: bool) -> Result<(), String> {
    let path = agent_path().ok_or("no home folder")?;
    if !on {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        };
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy().replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{AGENT}</string>
  <key>ProgramArguments</key><array><string>{exe}</string><string>{BACKGROUND_FLAG}</string></array>
  <key>RunAtLoad</key><true/>
  <key>LimitLoadToSessionType</key><string>Aqua</string>
  <key>ProcessType</key><string>Interactive</string>
</dict>
</plist>
"#
    );
    deskwave_core::config::write_atomic(&path, plist.as_bytes()).map_err(|e| e.to_string())
}

/// Keeps the sign-in entry pointing at this copy of the app if it was moved.
pub fn refresh_autostart() {
    if autostart_enabled() {
        let _ = set_autostart(true);
    }
}

fn open(target: &std::ffi::OsStr) -> Result<(), String> {
    let mut child = std::process::Command::new("/usr/bin/open").arg(target).spawn().map_err(|e| e.to_string())?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// Opens `url` in the default browser. Callers pass only fixed, known URLs.
pub fn open_url(url: &str) {
    let _ = open(url.as_ref());
}

/// Shows one of Deskwave's own folders in Finder.
pub fn open_folder(dir: &Path) -> Result<(), String> {
    open(dir.as_os_str())
}
