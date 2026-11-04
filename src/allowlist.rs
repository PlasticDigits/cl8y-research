//! Host, remote, and URL allowlists. Fail closed on SSRF and lookalikes.

use crate::error::{Error, Result};
use crate::invariants::{
    CANONICAL_ADDRESSES, COMPETITOR_WATCH_FORBIDDEN_HOSTS, COMPETITOR_WATCH_PAGES, INDEXER_HOSTS,
    POST_HREF_HOSTS, REPO_GIT_HOSTS, REPO_ORGS,
};
use url::Url;

const BLOCKED_HOSTS: &[&str] = &[
    "localhost",
    "127.0.0.1",
    "::1",
    "metadata.google.internal",
    "169.254.169.254",
];

pub fn is_blocked_ip_host(host: &str) -> bool {
    let h = host.trim_matches(['[', ']']).to_ascii_lowercase();
    if BLOCKED_HOSTS.contains(&h.as_str()) {
        return true;
    }
    if h.starts_with("10.") || h.starts_with("192.168.") || h.starts_with("0.") {
        return true;
    }
    if let Some(rest) = h.strip_prefix("172.") {
        if let Some((octet, _)) = rest.split_once('.') {
            if let Ok(n) = octet.parse::<u8>() {
                if (16..=31).contains(&n) {
                    return true;
                }
            }
        }
    }
    if h.starts_with("fd") || h.starts_with("fe80:") {
        return true;
    }
    false
}

pub fn parse_https_url(raw: &str) -> Result<Url> {
    let url = Url::parse(raw).map_err(|e| Error::Allowlist(e.to_string()))?;
    if url.scheme() != "https" {
        return Err(Error::Ssrf(format!(
            "only https allowed, got {}",
            url.scheme()
        )));
    }
    if url.username() != "" || url.password().is_some() {
        return Err(Error::Ssrf("userinfo not allowed".into()));
    }
    Ok(url)
}

pub fn host_of(url: &Url) -> Result<String> {
    let host = url
        .host_str()
        .ok_or_else(|| Error::Ssrf("missing host".into()))?;
    if host.contains("xn--") {
        return Err(Error::Ssrf(format!("punycode host rejected: {host}")));
    }
    if is_blocked_ip_host(host) {
        return Err(Error::Ssrf(format!("blocked host: {host}")));
    }
    if url.port().is_some() {
        return Err(Error::Ssrf("explicit ports are not allowlisted".into()));
    }
    Ok(host.to_ascii_lowercase())
}

/// Exact host match. `cl8y.com.attacker.tld` must not match `cl8y.com`.
pub fn host_allowed(host: &str, allow: &[&str]) -> bool {
    allow.iter().any(|h| host.eq_ignore_ascii_case(h))
}

pub fn reject_open_redirect(url: &Url) -> Result<()> {
    for (k, v) in url.query_pairs() {
        let key = k.to_ascii_lowercase();
        if matches!(
            key.as_str(),
            "url" | "redirect" | "next" | "return" | "goto"
        ) {
            return Err(Error::Allowlist(format!("open-redirect query {key}={v}")));
        }
    }
    Ok(())
}

pub fn check_fetch_url(raw: &str, allow: &[&str]) -> Result<Url> {
    let url = parse_https_url(raw)?;
    let host = host_of(&url)?;
    if !host_allowed(&host, allow) {
        return Err(Error::Allowlist(format!(
            "host {host} not in fetch allowlist"
        )));
    }
    reject_open_redirect(&url)?;
    Ok(url)
}

pub fn check_post_href(raw: &str) -> Result<Url> {
    if raw.starts_with('#') {
        // In-page anchors such as #token are allowed (directory lives on cl8y.com).
        return Url::parse("https://cl8y.com/").map_err(|e| Error::Allowlist(e.to_string()));
    }
    if raw.to_ascii_lowercase().starts_with("javascript:") {
        return Err(Error::Allowlist("javascript: href".into()));
    }
    let url = parse_https_url(raw)?;
    let host = host_of(&url)?;
    if !host_allowed(&host, POST_HREF_HOSTS) {
        return Err(Error::Allowlist(format!(
            "href host {host} not allowlisted"
        )));
    }
    reject_open_redirect(&url)?;
    Ok(url)
}

pub fn check_indexer_url(raw: &str) -> Result<Url> {
    check_fetch_url(raw, INDEXER_HOSTS)
}

/// Exact URL must match a committed competitor-watch page (not host-wide allow).
pub fn check_competitor_watch_url(raw: &str) -> Result<Url> {
    let url = parse_https_url(raw)?;
    let host = host_of(&url)?;
    if host_allowed(&host, COMPETITOR_WATCH_FORBIDDEN_HOSTS) {
        return Err(Error::Allowlist(format!(
            "competitor watch forbids host {host}"
        )));
    }
    reject_open_redirect(&url)?;
    let normalized = normalize_competitor_url(&url);
    if !COMPETITOR_WATCH_PAGES
        .iter()
        .any(|p| normalize_competitor_url_string(p.url) == normalized)
    {
        return Err(Error::Allowlist(format!(
            "url {raw} not in COMPETITOR_WATCH_PAGES"
        )));
    }
    Ok(url)
}

fn normalize_competitor_url_string(raw: &str) -> String {
    let url = Url::parse(raw).expect("committed competitor URLs must parse");
    normalize_competitor_url(&url)
}

fn normalize_competitor_url(url: &Url) -> String {
    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    let path = url.path();
    let path = if path.is_empty() { "/" } else { path };
    let mut out = format!("https://{host}{path}");
    if let Some(q) = url.query() {
        if !q.is_empty() {
            out.push('?');
            out.push_str(q);
        }
    }
    out
}

pub fn check_repo_remote(host_path: &str) -> Result<(String, String, String)> {
    let trimmed = host_path
        .trim()
        .trim_end_matches(".git")
        .trim_start_matches("https://");
    let mut parts = trimmed.split('/');
    let host = parts.next().unwrap_or("").to_string();
    let org = parts.next().unwrap_or("").to_string();
    let name = parts.next().unwrap_or("").to_string();
    if !REPO_GIT_HOSTS.iter().any(|h| host.eq_ignore_ascii_case(h)) {
        return Err(Error::Allowlist(format!("git host {host} not allowlisted")));
    }
    if is_blocked_ip_host(&host) {
        return Err(Error::Ssrf(format!("blocked git host {host}")));
    }
    if !REPO_ORGS.iter().any(|o| org.eq_ignore_ascii_case(o)) {
        return Err(Error::Allowlist(format!("org {org} not allowlisted")));
    }
    if name.is_empty() || parts.next().is_some() {
        return Err(Error::Allowlist(format!(
            "unexpected repo path {host_path}"
        )));
    }
    Ok((host, org, name))
}

pub fn looks_like_address(token: &str) -> bool {
    let t = token.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    if t.starts_with("terra1") && t.len() >= 20 {
        return true;
    }
    if t.starts_with("0x") && t.len() == 42 && t[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }
    false
}

pub fn is_canonical_address(token: &str) -> bool {
    CANONICAL_ADDRESSES
        .iter()
        .any(|a| a.eq_ignore_ascii_case(token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_metadata_and_private() {
        assert!(check_fetch_url("http://169.254.169.254/", INDEXER_HOSTS).is_err());
        assert!(check_fetch_url("https://169.254.169.254/", INDEXER_HOSTS).is_err());
        assert!(check_fetch_url("https://localhost/api", INDEXER_HOSTS).is_err());
        assert!(check_fetch_url("https://10.0.0.5/api", INDEXER_HOSTS).is_err());
    }

    #[test]
    fn rejects_lookalike_and_redirect() {
        assert!(check_post_href("https://bridge.cl8y.com.evil.example/").is_err());
        assert!(check_post_href("https://cl8y.com.attacker.tld/").is_err());
        assert!(check_post_href("https://xn--cl8y-com.example/").is_err());
        assert!(check_post_href("https://dex.cl8y.com/?url=https://evil.test").is_err());
        assert!(check_post_href("javascript:alert(1)").is_err());
    }

    #[test]
    fn allows_exact_product_hosts() {
        assert!(check_post_href("https://bridge.cl8y.com").is_ok());
        assert!(check_post_href("https://dex.cl8y.com/").is_ok());
        assert!(check_post_href("#token").is_ok());
    }

    #[test]
    fn rejects_untrusted_forks() {
        assert!(check_repo_remote("github.com/evil/yieldomega").is_err());
        assert!(check_repo_remote("github.com/PlasticDigits/yieldomega").is_ok());
    }
}
