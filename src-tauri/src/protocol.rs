//! `dwmod` protocol (`<MOD_ORIGIN>/<mod-id>/<path>` and `/_sdk/<path>`; see `sandbox::MOD_ORIGIN`).
//! Serves only approved mods, only files inside their folder, always with the sandbox CSP.

use crate::state::AppState;
use deskwave_core::manifest::resolve_in;
use deskwave_core::sandbox::{content_type, mod_csp, PERMISSIONS_POLICY};
use tauri::http::{header, Method, Request, Response, StatusCode};
use tauri::{Manager, Runtime, UriSchemeContext};

pub fn handle<R: Runtime>(ctx: UriSchemeContext<'_, R>, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    if request.method() != Method::GET {
        return status(StatusCode::METHOD_NOT_ALLOWED);
    }
    let state = ctx.app_handle().state::<AppState>();
    let path = request.uri().path().trim_start_matches('/');
    let (first, rest) = path.split_once('/').unwrap_or((path, ""));
    let (root, csp) = if first == "_sdk" {
        (state.paths.sdk.clone(), "default-src 'none'".to_owned())
    } else {
        match state.ready_mod(first) {
            Some(m) => (m.dir, mod_csp(first)),
            None => return status(StatusCode::FORBIDDEN),
        }
    };
    let Some(file) = resolve_in(&root, rest) else {
        return status(StatusCode::NOT_FOUND);
    };
    let Ok(body) = std::fs::read(&file) else {
        return status(StatusCode::NOT_FOUND);
    };
    Response::builder()
        .header(header::CONTENT_TYPE, content_type(&file))
        .header(header::CONTENT_SECURITY_POLICY, csp)
        .header("Permissions-Policy", PERMISSIONS_POLICY)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        // Mod frames have an opaque origin; module scripts need CORS to load at all.
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CACHE_CONTROL, "no-store")
        .body(body)
        .unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR))
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    let mut response = Response::new(Vec::new());
    *response.status_mut() = code;
    response
}
