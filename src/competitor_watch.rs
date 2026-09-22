//! Scheduled competitor fee/liquidity pages → raw notes (cl8y-research#14).

use crate::allowlist::check_competitor_watch_url;
use crate::error::{Error, Result};
use crate::invariants::{
    CompetitorPageFormat, CompetitorWatchPage, COMPETITOR_WATCH_BODY_CAP,
    COMPETITOR_WATCH_EXCERPT_CAP, COMPETITOR_WATCH_PAGES, COMPETITOR_WATCH_USER_AGENT,
};
use crate::numeric::NumericClaim;
use crate::replicate::wrap_untrusted;
use crate::store::{SourceKind, SourceRecord};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchCollection {
    pub collected_at: DateTime<Utc>,
    pub sources: Vec<SourceRecord>,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureCompetitorPage {
    pub id: String,
    pub status: u16,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
    #[serde(default)]
    pub body_text: Option<String>,
    #[serde(default)]
    pub skip_robots: bool,
}

pub fn collect_competitor_watch(
    pages: &[CompetitorWatchPage],
    fixtures: &[FixtureCompetitorPage],
    live: bool,
    now: DateTime<Utc>,
) -> Result<WatchCollection> {
    if pages.is_empty() {
        return Ok(WatchCollection {
            collected_at: now,
            sources: Vec::new(),
            gaps: vec!["no competitor pages configured".into()],
        });
    }

    let mut sources = Vec::new();
    let mut gaps = Vec::new();

    for (idx, page) in pages.iter().enumerate() {
        if idx > 0 && live {
            thread::sleep(Duration::from_millis(150));
        }
        if let Err(e) = check_competitor_watch_url(page.url) {
            return Err(Error::Ssrf(e.to_string()));
        }
        let fixture = fixtures.iter().find(|f| f.id == page.id);
        let outcome = if live {
            fetch_live_page(page)
        } else {
            match fixture {
                Some(f) => process_fixture_page(page, f),
                None => {
                    gaps.push(format!("fixture missing for page {}", page.id));
                    Ok(None)
                }
            }
        };

        match outcome {
            Ok(Some(rec)) => sources.push(rec),
            Ok(None) => {}
            Err(e) => gaps.push(format!("{}: {}", page.id, e)),
        }
    }

    Ok(WatchCollection {
        collected_at: now,
        sources,
        gaps,
    })
}

pub fn collect_competitor_watch_default(
    fixtures: &[FixtureCompetitorPage],
    live: bool,
    now: DateTime<Utc>,
) -> Result<WatchCollection> {
    collect_competitor_watch(COMPETITOR_WATCH_PAGES, fixtures, live, now)
}

fn process_fixture_page(
    page: &CompetitorWatchPage,
    fx: &FixtureCompetitorPage,
) -> Result<Option<SourceRecord>> {
    if fx.skip_robots {
        return Err(Error::Http("robots disallowed path".into()));
    }
    if fx.status == 429 || fx.status == 503 || fx.status == 404 || fx.status == 0 {
        return Err(Error::Http(format!("status {}", fx.status)));
    }
    if !fx.status.to_string().starts_with('2') {
        return Err(Error::Http(format!("status {}", fx.status)));
    }
    let body_bytes = fixture_body_bytes(page, fx)?;
    if body_bytes.len() > COMPETITOR_WATCH_BODY_CAP {
        return Err(Error::Http("body exceeds cap".into()));
    }
    extract_page(page, &body_bytes, now_utc())
}

fn fixture_body_bytes(page: &CompetitorWatchPage, fx: &FixtureCompetitorPage) -> Result<Vec<u8>> {
    match page.format {
        CompetitorPageFormat::Json => {
            let v = fx
                .body
                .clone()
                .ok_or_else(|| Error::Http("missing json body".into()))?;
            Ok(serde_json::to_vec(&v)?)
        }
        CompetitorPageFormat::Html => {
            let t = fx
                .body_text
                .clone()
                .ok_or_else(|| Error::Http("missing html body".into()))?;
            Ok(t.into_bytes())
        }
    }
}

fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

fn fetch_live_page(page: &CompetitorWatchPage) -> Result<Option<SourceRecord>> {
    let checked = check_competitor_watch_url(page.url)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| Error::Http(e.to_string()))?;
    let resp = client
        .get(checked.as_str())
        .header("User-Agent", COMPETITOR_WATCH_USER_AGENT)
        .header("Accept", page.accept)
        .send()
        .map_err(|e| Error::Http(e.to_string()))?;
    let status = resp.status();
    if status.as_u16() == 429 || status.as_u16() == 503 {
        return Err(Error::Http(format!("status {}", status)));
    }
    if !status.is_success() {
        return Err(Error::Http(format!("status {}", status)));
    }
    let bytes = resp
        .bytes()
        .map_err(|e| Error::Http(e.to_string()))?
        .to_vec();
    if bytes.len() > COMPETITOR_WATCH_BODY_CAP {
        return Err(Error::Http("body exceeds cap".into()));
    }
    extract_page(page, &bytes, Utc::now())
}

fn extract_page(
    page: &CompetitorWatchPage,
    body: &[u8],
    now: DateTime<Utc>,
) -> Result<Option<SourceRecord>> {
    let citation = format!("GET {}", page.url);
    let as_of = now.to_rfc3339();
    let (text, numbers, page_gaps) = match page.format {
        CompetitorPageFormat::Json => extract_json(page.extractor, body, &citation, &as_of, page)?,
        CompetitorPageFormat::Html => extract_html(page.extractor, body, &citation, &as_of, page)?,
    };
    if text.trim().is_empty() && numbers.is_empty() {
        if let Some(g) = page_gaps.first() {
            return Err(Error::Http(g.clone()));
        }
        return Err(Error::Http("extractor produced no fields".into()));
    }
    let wrapped = wrap_untrusted(&format!("competitor-watch:{}", page.id), &text);
    let gap = if page_gaps.is_empty() {
        None
    } else {
        Some(page_gaps.join("; "))
    };
    Ok(Some(SourceRecord {
        id: format!("competitor-watch:{}", page.id),
        kind: SourceKind::CompetitorWatch,
        collected_at: now,
        citation,
        text: wrapped,
        numbers,
        degraded: gap.is_some(),
        gap,
        metadata: serde_json::json!({
            "page_id": page.id,
            "extractor": page.extractor,
        }),
    }))
}

fn extract_json(
    extractor: &str,
    body: &[u8],
    citation: &str,
    as_of: &str,
    page: &CompetitorWatchPage,
) -> Result<(String, Vec<NumericClaim>, Vec<String>)> {
    let v: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| Error::Http(format!("json parse: {e}")))?;
    if json_contains_fetch_url(&v) {
        return Err(Error::Ssrf("json contained fetchable URL".into()));
    }
    match extractor {
        "defillama_protocol" => extract_defillama_protocol(&v, citation, as_of, page),
        _ => Err(Error::Http(format!("unknown json extractor {extractor}"))),
    }
}

fn json_contains_fetch_url(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Object(m) => {
            for (k, val) in m {
                if matches!(k.as_str(), "next" | "redirect" | "goto") {
                    if let Some(s) = val.as_str() {
                        let lower = s.to_ascii_lowercase();
                        if lower.starts_with("http://") || lower.starts_with("https://") {
                            return true;
                        }
                    }
                }
                if json_contains_fetch_url(val) {
                    return true;
                }
            }
            false
        }
        serde_json::Value::Array(a) => a.iter().any(json_contains_fetch_url),
        _ => false,
    }
}

fn extract_defillama_protocol(
    v: &serde_json::Value,
    citation: &str,
    as_of: &str,
    page: &CompetitorWatchPage,
) -> Result<(String, Vec<NumericClaim>, Vec<String>)> {
    let mut gaps = Vec::new();
    let mut numbers = Vec::new();
    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or(page.id);
    let mut parts = vec![format!(
        "Competitor watch {name} (untrusted, not CL8Y indexer)"
    )];

    if let Some(fee) = v.get("fee_bps").and_then(json_number_string) {
        parts.push(format!("fee_bps {fee}"));
        numbers.push(claim(&fee, citation, as_of, page));
    } else {
        gaps.push("fee_bps missing".into());
    }

    let liquidity = liquidity_from_defillama(v);
    if let Some(liq) = liquidity {
        parts.push(format!("liquidity_or_tvl_usd {liq}"));
        numbers.push(NumericClaim {
            value: liq,
            citation: citation.to_string(),
            as_of: as_of.to_string(),
            source_id: format!("competitor-watch:{}", page.id),
        });
    } else {
        gaps.push("liquidity_or_tvl missing".into());
    }

    if let Some(vol) = v
        .pointer("/volume_24h")
        .and_then(json_number_string)
        .or_else(|| v.get("volume_24h").and_then(json_number_string))
    {
        parts.push(format!("volume_24h {vol}"));
        numbers.push(NumericClaim {
            value: vol,
            citation: citation.to_string(),
            as_of: as_of.to_string(),
            source_id: format!("competitor-watch:{}", page.id),
        });
    }

    let excerpt = serde_json::to_string(v).unwrap_or_default();
    let excerpt = truncate_excerpt(&excerpt);
    parts.push(format!("raw_excerpt {excerpt}"));

    Ok((parts.join(". "), numbers, gaps))
}

fn claim(value: &str, citation: &str, as_of: &str, page: &CompetitorWatchPage) -> NumericClaim {
    NumericClaim {
        value: value.to_string(),
        citation: citation.to_string(),
        as_of: as_of.to_string(),
        source_id: format!("competitor-watch:{}", page.id),
    }
}

fn liquidity_from_defillama(v: &serde_json::Value) -> Option<String> {
    if let Some(cc) = v.get("currentChainTvls").and_then(|x| x.as_object()) {
        let sum: f64 = cc.values().filter_map(|x| x.as_f64()).sum();
        if sum > 0.0 {
            return Some(format_compact_usd(sum));
        }
    }
    if let Some(series) = v.get("tvl").and_then(|x| x.as_array()) {
        if let Some(last) = series.last() {
            if let Some(liq) = last.get("totalLiquidityUSD").and_then(|x| x.as_f64()) {
                return Some(format_compact_usd(liq));
            }
        }
    }
    None
}

fn format_compact_usd(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{:.0}", n)
    } else {
        format!("{:.2}", n)
    }
}

fn json_number_string(v: &serde_json::Value) -> Option<String> {
    if let Some(n) = v.as_u64() {
        return Some(n.to_string());
    }
    if let Some(n) = v.as_f64() {
        return Some(format_compact_usd(n));
    }
    v.as_str().map(|s| s.to_string())
}

fn extract_html(
    extractor: &str,
    body: &[u8],
    citation: &str,
    as_of: &str,
    page: &CompetitorWatchPage,
) -> Result<(String, Vec<NumericClaim>, Vec<String>)> {
    let raw = decode_body_text(body);
    let stripped = strip_html_for_excerpt(&raw);
    match extractor {
        "html_fee_bps" => {
            let mut gaps = Vec::new();
            let fee = extract_html_fee_bps(&raw);
            let mut numbers = Vec::new();
            let mut parts = vec!["Competitor HTML fee page (untrusted)".to_string()];
            if let Some(fee) = fee {
                parts.push(format!("fee_bps {fee}"));
                numbers.push(claim(&fee, citation, as_of, page));
            } else {
                gaps.push("fee_bps missing".into());
            }
            let excerpt = truncate_excerpt(&stripped);
            parts.push(format!("raw_excerpt {excerpt}"));
            if stripped.to_ascii_lowercase().contains("<script") {
                gaps.push("script tags present (not executed)".into());
            }
            Ok((parts.join(". "), numbers, gaps))
        }
        _ => Err(Error::Http(format!("unknown html extractor {extractor}"))),
    }
}

fn extract_html_fee_bps(html: &str) -> Option<String> {
    if let Some(start) = html.find("id=\"fee-bps\"") {
        let slice = &html[start..];
        if let Some(gt) = slice.find('>') {
            let after = &slice[gt + 1..];
            if let Some(end) = after.find('<') {
                let val = after[..end].trim();
                if !val.is_empty() {
                    return Some(val.replace(',', ""));
                }
            }
        }
    }
    if let Some(start) = html.find("data-fee-bps=\"") {
        let rest = &html[start + 14..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn strip_html_for_excerpt(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut out = String::new();
    let mut in_script = false;
    let mut in_style = false;
    let mut i = 0;
    while i < html.len() {
        if lower[i..].starts_with("<script") {
            in_script = true;
        }
        if lower[i..].starts_with("</script") {
            in_script = false;
            i += 1;
            continue;
        }
        if lower[i..].starts_with("<style") {
            in_style = true;
        }
        if lower[i..].starts_with("</style") {
            in_style = false;
            i += 1;
            continue;
        }
        if !in_script && !in_style {
            if let Some(ch) = html[i..].chars().next() {
                if ch != '<' {
                    out.push(ch);
                }
                i += ch.len_utf8();
                continue;
            }
        }
        i += 1;
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_body_text(body: &[u8]) -> String {
    String::from_utf8_lossy(body).to_string()
}

fn truncate_excerpt(s: &str) -> String {
    if s.len() <= COMPETITOR_WATCH_EXCERPT_CAP {
        s.to_string()
    } else {
        let mut t = s[..COMPETITOR_WATCH_EXCERPT_CAP].to_string();
        t.push('…');
        t
    }
}

pub fn load_fixture_competitor_pages(dir: &Path) -> Result<Vec<FixtureCompetitorPage>> {
    let path = dir.join("sources.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)?;
    let v: serde_json::Value = serde_json::from_str(&raw)?;
    if let Some(arr) = v.get("competitor_watch").and_then(|x| x.as_array()) {
        return Ok(serde_json::from_value(serde_json::Value::Array(
            arr.clone(),
        ))?);
    }
    Ok(Vec::new())
}

pub fn write_watch_artifacts(out: &Path, watch: &WatchCollection) -> Result<()> {
    std::fs::create_dir_all(out)?;
    let notes_path = out.join("notes.json");
    std::fs::write(&notes_path, serde_json::to_string_pretty(&watch.sources)?)?;
    let gaps_body = if watch.gaps.is_empty() {
        "# Gaps\n\n(none)\n".to_string()
    } else {
        format!(
            "# Gaps\n\n{}\n",
            watch
                .gaps
                .iter()
                .map(|g| format!("- {g}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    std::fs::write(out.join("gaps.md"), gaps_body)?;
    let sources = serde_json::json!({
        "collected_at": watch.collected_at,
        "competitor_watch_ids": watch.sources.iter().map(|s| s.id.clone()).collect::<Vec<_>>(),
    });
    std::fs::write(
        out.join("sources.json"),
        serde_json::to_string_pretty(&sources)?,
    )?;
    Ok(())
}

pub fn load_watch_notes(path: &Path) -> Result<Vec<SourceRecord>> {
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

/// Numeric claims usable for week emit (excludes competitor-watch inbox figures).
pub fn claims_for_week_emit(sources: &[SourceRecord]) -> Vec<NumericClaim> {
    sources
        .iter()
        .filter(|s| s.kind.onchain_authoritative())
        .flat_map(|s| s.numbers.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invariants::CompetitorPageFormat;

    fn html_page() -> CompetitorWatchPage {
        CompetitorWatchPage {
            id: "html-test",
            url: "https://api.llama.fi/protocol/terraswap",
            format: CompetitorPageFormat::Html,
            extractor: "html_fee_bps",
            accept: "text/html",
        }
    }

    #[test]
    fn html_extractor_ignores_script_in_claims() {
        let html = r#"<html><body><div id="fee-bps">30</div><script>fetch('https://evil')</script></body></html>"#;
        let page = html_page();
        let (text, nums, _) =
            extract_html("html_fee_bps", html.as_bytes(), "GET x", "t", &page).unwrap();
        assert_eq!(nums[0].value, "30");
        assert!(!text.contains("fetch("));
    }

    #[test]
    fn defillama_null_tvl_gaps_but_fee_stays() {
        let body = serde_json::json!({"name": "Test", "fee_bps": 25, "currentChainTvls": null});
        let page = COMPETITOR_WATCH_PAGES[0];
        let (_, nums, gaps) = extract_json(
            "defillama_protocol",
            serde_json::to_vec(&body).unwrap().as_slice(),
            "GET u",
            "t",
            &page,
        )
        .unwrap();
        assert!(gaps.iter().any(|g| g.contains("liquidity")));
        assert!(nums.iter().any(|n| n.value == "25"));
    }
}
