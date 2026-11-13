//! The network broker: the only code in Deskwave that talks to the internet. Callers check
//! the internet switch, the mod's grant and the URL (deskwave_core::net) before calling fetch.

use deskwave_core::net::{is_public, MAX_RESPONSE};
use serde::Serialize;
use std::sync::OnceLock;
use std::time::Duration;
use ureq::config::Config;
use ureq::http::Uri;
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};
use ureq::Agent;

/// Refuses names that resolve to the user's own network (DNS rebinding).
struct PublicOnly(DefaultResolver);

impl std::fmt::Debug for PublicOnly {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PublicOnly")
    }
}

impl Resolver for PublicOnly {
    fn resolve(&self, uri: &Uri, config: &Config, timeout: NextTimeout) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let addrs = self.0.resolve(uri, config, timeout)?;
        if addrs.iter().all(|a| is_public(a.ip())) {
            Ok(addrs)
        } else {
            Err(ureq::Error::HostNotFound)
        }
    }
}

fn agent() -> &'static Agent {
    static AGENT: OnceLock<Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        let config = Agent::config_builder()
            .https_only(true)
            // A redirect could lead to a host the user never approved.
            .max_redirects(0)
            .max_redirects_will_error(false)
            .http_status_as_error(false)
            // shortcut: direct connections only; a proxy would resolve names itself and skip
            // the private-address check. Add proxy support with its own check if users need it.
            .proxy(None)
            .timeout_global(Some(Duration::from_secs(10)))
            .user_agent("Deskwave")
            .build();
        Agent::with_parts(config, DefaultConnector::default(), PublicOnly(DefaultResolver::default()))
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reply {
    status: u16,
    content_type: String,
    body: String,
}

impl Reply {
    pub fn status(&self) -> u16 {
        self.status
    }
}

/// GET `url` (already checked) and return its text. No cookies, no credentials.
pub fn fetch(url: &str) -> Result<Reply, String> {
    let mut response = agent()
        .get(url)
        .header("Accept", "application/json, text/plain;q=0.9, */*;q=0.1")
        .call()
        .map_err(describe)?;
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let body = response.body_mut().with_config().limit(MAX_RESPONSE).read_to_string().map_err(describe)?;
    Ok(Reply { status, content_type, body })
}

/// Error codes the pages translate.
fn describe(error: ureq::Error) -> String {
    match error {
        ureq::Error::HostNotFound => "unreachable",
        ureq::Error::Timeout(_) => "timeout",
        ureq::Error::BodyExceedsLimit(_) => "too-large",
        _ => "network-error",
    }
    .into()
}
