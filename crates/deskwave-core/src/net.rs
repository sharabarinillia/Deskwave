//! Rules for the network broker, the only code in Deskwave that talks to the internet.

use http::Uri;
use std::net::IpAddr;

pub const MAX_URL: usize = 2048;
pub const MAX_RESPONSE: u64 = 1024 * 1024;

/// Accepts `url` only if it is plain HTTPS to one of `allowed` (exact host, default port,
/// no credentials). Parsed with the same `http::Uri` the HTTP client uses, so both agree on
/// which host is meant. Returns the host, or an error code the pages translate.
pub fn check_url(url: &str, allowed: &[String]) -> Result<String, String> {
    if url.len() > MAX_URL || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("bad-url".into());
    }
    let uri: Uri = url.parse().map_err(|_| "bad-url")?;
    let authority = uri.authority().ok_or("bad-url")?;
    if uri.scheme_str() != Some("https")
        || authority.as_str().contains('@')
        || authority.port().is_some_and(|p| p.as_u16() != 443)
    {
        return Err("bad-url".into());
    }
    let host = authority.host().to_ascii_lowercase();
    if !allowed.contains(&host) {
        return Err("host-not-allowed".into());
    }
    Ok(host)
}

/// True for addresses on the public internet. Used after DNS resolution, so an approved
/// name that resolves to the user's own network (DNS rebinding) is still refused.
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                || a == 0
                || a >= 240
                || (a == 100 && (64..128).contains(&b))) // carrier-grade NAT
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || first & 0xfe00 == 0xfc00 // unique local
                || first & 0xffc0 == 0xfe80) // link local
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_must_be_https_to_an_approved_host() {
        let allowed = vec!["api.open-meteo.com".to_string()];
        assert_eq!(check_url("https://api.open-meteo.com/v1/forecast?latitude=1", &allowed).unwrap(), "api.open-meteo.com");
        assert!(check_url("https://API.Open-Meteo.com/v1", &allowed).is_ok());
        assert!(check_url("https://api.open-meteo.com:443/v1", &allowed).is_ok());
        for bad in [
            "http://api.open-meteo.com/v1",
            "https://evil.com/v1",
            "https://api.open-meteo.com.evil.com/",
            "https://api.open-meteo.com@evil.com/",
            "https://user@api.open-meteo.com/",
            "https://api.open-meteo.com:8443/",
            "https://evil.com/?https://api.open-meteo.com",
            "https://api.open-meteo.com /x",
            "//api.open-meteo.com/",
            "file:///C:/Windows",
        ] {
            assert!(check_url(bad, &allowed).is_err(), "{bad}");
        }
    }

    #[test]
    fn private_addresses_are_refused() {
        for ip in ["8.8.8.8", "1.1.1.1", "2606:4700::1111"] {
            assert!(is_public(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["127.0.0.1", "10.0.0.1", "172.16.0.1", "192.168.1.1", "169.254.1.1", "100.64.0.1",
                   "0.0.0.0", "255.255.255.255", "::1", "fd00::1", "fe80::1", "::ffff:192.168.1.1"] {
            assert!(!is_public(ip.parse().unwrap()), "{ip}");
        }
    }
}
