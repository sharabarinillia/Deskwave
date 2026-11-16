//! Headers and browser flags that confine mods. See docs/SPEC.md, "Sandbox".

use std::path::Path;

/// Where Tauri serves the `dwmod` protocol: WebView2 (Windows) uses `http://<scheme>.localhost`,
/// WebKit (macOS) uses `<scheme>://localhost`.
pub const MOD_ORIGIN: &str = if cfg!(windows) { "http://dwmod.localhost" } else { "dwmod://localhost" };

/// CSP for every file of mod `id`: only its own folder and the SDK, no network, and the
/// document is sandboxed even if something loads it outside our sandboxed iframe.
pub fn mod_csp(id: &str) -> String {
    let own = format!("{MOD_ORIGIN}/{id}/");
    let sdk = format!("{MOD_ORIGIN}/_sdk/");
    format!(
        "default-src 'none'; script-src {own} {sdk}; style-src {own} {sdk} 'unsafe-inline'; \
         img-src {own} data: blob:; font-src {own} {sdk}; media-src {own} blob:; \
         connect-src 'none'; frame-src 'none'; worker-src 'none'; object-src 'none'; \
         form-action 'none'; base-uri 'none'; sandbox allow-scripts"
    )
}

pub const PERMISSIONS_POLICY: &str = "camera=(), microphone=(), geolocation=(), \
    display-capture=(), usb=(), hid=(), serial=(), bluetooth=(), payment=(), \
    clipboard-read=(), publickey-credentials-get=()";

/// Flags for every WebView2 in the app. WebView2 requires them to be identical across
/// windows. The proxy points at a port nothing listens on, so page-level network requests
/// fail even if a CSP gap existed; real internet access will go through the Rust broker.
/// The `--disable-features` list keeps Tauri's defaults, which this string replaces.
pub const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
    --proxy-server=http://127.0.0.1:9 --force-webrtc-ip-handling-policy=disable_non_proxied_udp \
    --disable-background-networking --no-pings";

pub fn content_type(path: &Path) -> &'static str {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "apng" => "image/apng",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csp_confines_mod_to_its_folder() {
        let csp = mod_csp("acme.clock");
        assert!(csp.contains("connect-src 'none'"));
        assert!(csp.contains(&format!("script-src {MOD_ORIGIN}/acme.clock/ {MOD_ORIGIN}/_sdk/;")));
        assert!(csp.ends_with("sandbox allow-scripts"));
        assert!(!csp.contains("unsafe-eval") && !csp.contains("script-src 'self'"));
    }

    #[test]
    fn content_types() {
        assert_eq!(content_type(Path::new("a/INDEX.HTML")), "text/html; charset=utf-8");
        assert_eq!(content_type(Path::new("x.exe")), "application/octet-stream");
    }
}
