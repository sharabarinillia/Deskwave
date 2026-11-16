//! Windows: putting windows into the desktop, and reading the system state used for pausing.
//! The desktop window tree is undocumented; see docs/SPEC.md, "Desktop integration".

use super::{Coverage, Power, BACKGROUND_FLAG};
use crate::state::Monitor;
use deskwave_core::config::KeepAwake;
use tauri::{AppHandle, WebviewWindow};
use windows::core::{w, BOOL, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, COLORREF, ERROR_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS,
    HANDLE, HWND, LPARAM, POINT, RECT, WAIT_OBJECT_0, WPARAM,
};
use windows::core::{HSTRING, PWSTR};
use windows::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_DWORD,
    RRF_RT_REG_SZ,
};
use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, OpenInputDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_SWITCHDESKTOP,
};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, GetCurrentProcessId, OpenEventW, SetEvent, WaitForSingleObject,
    EVENT_MODIFY_STATE,
};
use windows::Win32::UI::WindowsAndMessaging::*;

pub const PLATFORM: &str = "windows";
/// The Now playing widget reads the Windows media session.
pub const MEDIA: bool = true;

/// Window handles cross threads as plain integers.
pub type Hwnd = isize;

fn hwnd(h: Hwnd) -> HWND {
    HWND(h as *mut _)
}

fn hwnd_of(window: &WebviewWindow) -> Option<Hwnd> {
    window.hwnd().ok().map(|h| h.0 as Hwnd)
}

/// The event a later launch signals to ask this one to show Settings (0 if it couldn't be made).
pub struct Instance(Hwnd);

/// `None` if Deskwave is already running; that copy is then asked to open Settings.
/// The mutex lives until the process exits.
pub fn single_instance() -> Option<Instance> {
    unsafe {
        let created = CreateMutexW(None, true, w!("Local\\Deskwave.SingleInstance"));
        if created.is_err() || GetLastError() == ERROR_ALREADY_EXISTS {
            if let Ok(event) = OpenEventW(EVENT_MODIFY_STATE, false, w!("Local\\Deskwave.ShowSettings")) {
                let _ = SetEvent(event);
                let _ = CloseHandle(event);
            }
            return None;
        }
        let event = CreateEventW(None, false, false, w!("Local\\Deskwave.ShowSettings"));
        Some(Instance(event.map_or(0, |h| h.0 as Hwnd)))
    }
}

impl Instance {
    /// True once each time another launch asked us to show Settings.
    pub fn take_show_request(&self) -> bool {
        self.0 != 0 && unsafe { WaitForSingleObject(HANDLE(self.0 as *mut _), 0) == WAIT_OBJECT_0 }
    }
}

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

/// Keeps the sign-in entry pointing at this exe with the current flags, and replaces the
/// entry from when the app was called ColorBy.
pub fn refresh_autostart() {
    let legacy = unsafe { RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, w!("ColorBy"), RRF_RT_REG_SZ, None, None, None).is_ok() };
    if legacy {
        unsafe {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, w!("ColorBy"));
        }
    }
    if legacy || autostart_enabled() {
        let _ = set_autostart(true);
    }
}

pub fn autostart_enabled() -> bool {
    unsafe { RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, w!("Deskwave"), RRF_RT_REG_SZ, None, None, None).is_ok() }
}

/// Windows display languages in order of preference, e.g. `["uk-UA", "en-US"]`.
pub fn ui_languages() -> Vec<String> {
    let (mut count, mut len) = (0u32, 0u32);
    unsafe {
        if GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, None, &mut len).is_err() {
            return Vec::new();
        }
        let mut buffer = vec![0u16; len as usize];
        if GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, Some(PWSTR(buffer.as_mut_ptr())), &mut len).is_err() {
            return Vec::new();
        }
        String::from_utf16_lossy(&buffer).split('\0').filter(|s| !s.is_empty()).map(str::to_owned).collect()
    }
}

/// Opens `url` in the default browser. Callers pass only fixed, known URLs.
pub fn open_url(url: &str) {
    unsafe {
        ShellExecuteW(None, w!("open"), &HSTRING::from(url), None, None, SW_SHOWNORMAL);
    }
}

/// Shows one of Deskwave's own folders in File Explorer.
pub fn open_folder(dir: &std::path::Path) -> Result<(), String> {
    std::process::Command::new("explorer.exe").arg(dir).spawn().map(drop).map_err(|e| e.to_string())
}

/// WebView2 gets its hardening from the browser arguments (`sandbox::BROWSER_ARGS`).
pub fn harden_webview(_window: &WebviewWindow) {}

/// Keeping awake is a macOS feature; Windows has its own power settings for this.
pub fn set_keep_awake(_mode: KeepAwake) -> Result<(), String> {
    Ok(())
}

/// Seconds since the last keyboard or mouse input; only used on macOS.
pub fn idle_seconds() -> u64 {
    0
}

/// Adds or removes Deskwave from the current user's sign-in programs.
pub fn set_autostart(on: bool) -> Result<(), String> {
    let result = unsafe {
        if on {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let value: Vec<u16> =
                format!("\"{}\" {BACKGROUND_FLAG}", exe.display()).encode_utf16().chain([0]).collect();
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                RUN_KEY,
                w!("Deskwave"),
                REG_SZ.0,
                Some(value.as_ptr().cast()),
                (value.len() * 2) as u32,
            )
        } else {
            match RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, w!("Deskwave")) {
                ERROR_FILE_NOT_FOUND => ERROR_SUCCESS,
                other => other,
            }
        }
    };
    result.ok().map_err(|e| e.to_string())
}

fn find_child(parent: Option<HWND>, after: Option<HWND>, class: PCWSTR) -> Option<HWND> {
    unsafe { FindWindowExW(parent, after, class, PCWSTR::null()).ok() }
}

/// The top-level window that hosts the desktop icons: `Progman` on 24H2+ (and before the
/// split), otherwise the `WorkerW` Explorer created for it.
fn icons_host() -> Option<HWND> {
    unsafe extern "system" fn visit(top: HWND, out: LPARAM) -> BOOL {
        if find_child(Some(top), None, w!("SHELLDLL_DefView")).is_some() {
            *(out.0 as *mut HWND) = top;
            return false.into();
        }
        true.into()
    }
    let mut found = HWND::default();
    unsafe {
        // Err only means the callback stopped early.
        let _ = EnumWindows(Some(visit), LPARAM(&mut found as *mut HWND as isize));
    }
    (!found.is_invalid()).then_some(found)
}

fn progman() -> Option<HWND> {
    unsafe { FindWindowW(w!("Progman"), PCWSTR::null()).ok() }
}

/// Owner for widget layers: keeps them above the wallpaper, below apps, and visible on Win+D.
pub fn desktop_owner() -> Option<Hwnd> {
    icons_host().map(|h| h.0 as Hwnd)
}

fn rect_of(m: &Monitor) -> RECT {
    RECT { left: m.rect.0, top: m.rect.1, right: m.rect.0 + m.rect.2 as i32, bottom: m.rect.1 + m.rect.3 as i32 }
}

/// Puts a new wallpaper window into the desktop, below the icons, covering monitor `m`.
/// Shown via Win32: tao's show() would restore the popup styles.
pub fn place_wallpaper(window: &WebviewWindow, m: &Monitor) -> Result<(), String> {
    let handle = hwnd_of(window).ok_or("no window handle")?;
    attach_wallpaper(handle, rect_of(m)).map(drop)
}

/// Shows a new widget layer above the wallpaper and below other apps.
pub fn place_widgets(window: &WebviewWindow, owner: Option<Hwnd>) -> tauri::Result<()> {
    if let (Some(owner), Some(handle)) = (owner, hwnd_of(window)) {
        set_owner(handle, owner);
    }
    window.show()?;
    sink(window, owner);
    Ok(())
}

/// Edit mode lifts a widget layer above all windows; leaving it sinks the layer back.
pub fn set_editing(window: &WebviewWindow, on: bool, owner: Option<Hwnd>) {
    let _ = window.set_always_on_top(on);
    if !on {
        sink(window, owner);
    }
}

/// Parents `window` into the desktop below the icons and stretches it over `rect` (physical
/// screen pixels). Returns the parent so the caller can notice when Explorer restarts.
fn attach_wallpaper(window: Hwnd, rect: RECT) -> Result<Hwnd, String> {
    let window = hwnd(window);
    let progman = progman().ok_or("Progman not found")?;
    unsafe {
        // 0x052C asks Explorer to split the wallpaper from the icons (0xD, 1 as Lively and
        // others send; the 0, 0 form depends on the animation setting).
        let split = |w: usize, l: isize| {
            SendMessageTimeoutW(progman, 0x052C, WPARAM(w), LPARAM(l), SMTO_NORMAL, 1000, None)
        };
        split(0xD, 1);
        let mut parent = None;
        for attempt in 0..10 {
            if let Some(host) = icons_host().filter(|h| *h != progman) {
                // Classic: the wallpaper WorkerW sits right behind the icons host.
                parent = find_child(None, Some(host), w!("WorkerW"));
            } else if raised(progman) {
                parent = Some(progman);
            }
            if parent.is_some() {
                break;
            }
            if attempt == 2 {
                split(0, 0);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        // Fallback for desktops that never split: draw inside Progman under the icons.
        let parent = parent.unwrap_or(progman);
        let in_progman = parent == progman;

        let style = GetWindowLongPtrW(window, GWL_STYLE) as u32;
        let strip = WS_POPUP.0 | WS_CAPTION.0 | WS_THICKFRAME.0 | WS_SYSMENU.0
            | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0;
        SetWindowLongPtrW(window, GWL_STYLE, ((style & !strip) | WS_CHILD.0) as isize);
        let mut ex = GetWindowLongPtrW(window, GWL_EXSTYLE) as u32 | WS_EX_NOACTIVATE.0;
        if in_progman {
            // 24H2+: Progman has no redirection surface; only layered children get composited.
            ex |= WS_EX_LAYERED.0;
        }
        SetWindowLongPtrW(window, GWL_EXSTYLE, ex as isize);
        if in_progman {
            SetLayeredWindowAttributes(window, COLORREF(0), 255, LWA_ALPHA).map_err(|e| e.to_string())?;
        }
        SetParent(window, Some(parent)).map_err(|e| e.to_string())?;

        let mut origin = POINT { x: rect.left, y: rect.top };
        let _ = ScreenToClient(parent, &mut origin);
        // Directly below the icon view when sharing Progman with it; otherwise on top.
        let after = if in_progman {
            find_child(Some(progman), None, w!("SHELLDLL_DefView"))
        } else {
            Some(HWND_TOP)
        };
        SetWindowPos(
            window,
            after,
            origin.x,
            origin.y,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .map_err(|e| e.to_string())?;
        if in_progman {
            // Explorer's own wallpaper WorkerW must stay behind ours.
            if let Some(worker) = find_child(Some(progman), None, w!("WorkerW")) {
                let _ = SetWindowPos(
                    worker,
                    Some(HWND_BOTTOM),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
        Ok(parent.0 as Hwnd)
    }
}

fn raised(progman: HWND) -> bool {
    unsafe {
        let ex = GetWindowLongPtrW(progman, GWL_EXSTYLE) as u32;
        ex & WS_EX_NOREDIRECTIONBITMAP.0 != 0
            && find_child(Some(progman), None, w!("SHELLDLL_DefView")).is_some()
    }
}

/// Makes `owner` own `window`; owned windows stay above their owner and leave Alt+Tab.
fn set_owner(window: Hwnd, owner: Hwnd) {
    unsafe {
        SetWindowLongPtrW(hwnd(window), GWLP_HWNDPARENT, owner);
    }
}

/// Moves a widget layer back down to just above the desktop, without taking focus away.
pub fn sink(window: &WebviewWindow, owner: Option<Hwnd>) {
    if let (Some(owner), Some(handle)) = (owner, hwnd_of(window)) {
        sink_hwnd(handle, owner);
    }
}

fn sink_hwnd(window: Hwnd, owner: Hwnd) {
    unsafe {
        if let Ok(above_owner) = GetWindow(hwnd(owner), GW_HWNDPREV) {
            if above_owner != hwnd(window) {
                let _ = SetWindowPos(
                    hwnd(window),
                    Some(above_owner),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }
}

/// Makes Explorer repaint the static wallpaper after our windows are gone.
pub fn repaint_static_wallpaper() {
    let mut path = [0u16; 260];
    unsafe {
        if SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            path.len() as u32,
            Some(path.as_mut_ptr().cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
        {
            let _ = SystemParametersInfoW(
                SPI_SETDESKWALLPAPER,
                0,
                Some(path.as_mut_ptr().cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );
        }
    }
}

/// Cursor position in physical screen pixels.
pub fn cursor_pos(_app: &AppHandle) -> (f64, f64) {
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    (p.x as f64, p.y as f64)
}

/// Windows accent color as `#rrggbb` (stored as 0xAABBGGRR).
pub fn accent_color() -> String {
    let mut value = 0u32;
    let mut size = 4u32;
    let ok = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\DWM"),
            w!("AccentColor"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut size),
        )
        .is_ok()
    };
    if !ok {
        return "#0067c0".into();
    }
    let [r, g, b, _] = value.to_le_bytes();
    format!("#{r:02x}{g:02x}{b:02x}")
}

pub fn power() -> Power {
    let mut s = SYSTEM_POWER_STATUS::default();
    let _ = unsafe { GetSystemPowerStatus(&mut s) };
    Power {
        present: s.BatteryFlag != 128 && s.BatteryFlag != 255,
        percent: (s.BatteryLifePercent <= 100).then_some(s.BatteryLifePercent),
        charging: s.BatteryFlag != 255 && s.BatteryFlag & 8 != 0,
        on_ac: s.ACLineStatus != 0,
        saver: s.SystemStatusFlag == 1,
        seconds_left: (s.BatteryLifeTime != u32::MAX).then_some(s.BatteryLifeTime),
    }
}

/// Locked screen (the input desktop is not ours) or a Remote Desktop session.
pub fn session_inactive() -> bool {
    unsafe {
        let locked = match OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_SWITCHDESKTOP) {
            Ok(desk) => {
                let _ = CloseDesktop(desk);
                false
            }
            Err(_) => true,
        };
        locked || GetSystemMetrics(SM_REMOTESESSION) != 0
    }
}

/// What hides the wallpaper on each monitor. Skips our own windows, the shell desktop,
/// minimized/cloaked windows and click-through/tool overlays (e.g. GPU driver overlays).
pub fn coverage(_app: &AppHandle, monitors: &[Monitor]) -> Vec<Coverage> {
    let rects: Vec<RECT> = monitors.iter().map(rect_of).collect();
    covering(&rects)
}

fn covering(monitors: &[RECT]) -> Vec<Coverage> {
    struct Ctx<'a> {
        monitors: &'a [RECT],
        out: Vec<Coverage>,
        pid: u32,
    }
    unsafe extern "system" fn visit(h: HWND, data: LPARAM) -> BOOL {
        let ctx = &mut *(data.0 as *mut Ctx);
        let mut pid = 0;
        GetWindowThreadProcessId(h, Some(&mut pid));
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4);
        let mut class = [0u16; 32];
        let len = GetClassNameW(h, &mut class) as usize;
        let class = String::from_utf16_lossy(&class[..len]);
        if pid == ctx.pid
            || !IsWindowVisible(h).as_bool()
            || IsIconic(h).as_bool()
            || cloaked != 0
            || ex & (WS_EX_TOOLWINDOW.0 | WS_EX_TRANSPARENT.0 | WS_EX_NOACTIVATE.0) != 0
            || class == "Progman"
            || class == "WorkerW"
        {
            return true.into();
        }
        let mut r = RECT::default();
        if GetWindowRect(h, &mut r).is_err() {
            return true.into();
        }
        let (cx, cy) = ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
        let maximized = IsZoomed(h).as_bool();
        for (m, cov) in ctx.monitors.iter().zip(ctx.out.iter_mut()) {
            if cx >= m.left && cx < m.right && cy >= m.top && cy < m.bottom {
                cov.maximized |= maximized;
                cov.fullscreen |= r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom;
            }
        }
        true.into()
    }
    let mut ctx = Ctx { monitors, out: vec![Coverage::default(); monitors.len()], pid: unsafe { GetCurrentProcessId() } };
    unsafe {
        let _ = EnumWindows(Some(visit), LPARAM(&mut ctx as *mut Ctx as isize));
    }
    ctx.out
}
