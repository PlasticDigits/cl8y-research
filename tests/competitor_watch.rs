//! cl8y-research#14 competitor watch functional + abuse tests.

use chrono::TimeZone;
use cl8y_research::allowlist::check_competitor_watch_url;
use cl8y_research::collect::collect_from_fixture;
use cl8y_research::competitor_watch::claims_for_week_emit;
use cl8y_research::competitor_watch::{
    collect_competitor_watch, collect_competitor_watch_default, write_watch_artifacts,
    FixtureCompetitorPage,
};
use cl8y_research::config::Config;
use cl8y_research::embed::{Embedder, HashingEmbedder};
use cl8y_research::invariants::{
    CompetitorPageFormat, CompetitorWatchPage, COMPETITOR_WATCH_PAGES,
};
use cl8y_research::numeric::unsourced;
use cl8y_research::store::{
    parse_stored_kind, DocumentStore, MemoryStore, SourceKind, SourceRecord,
};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn ac1_competitor_kind_round_trip() {
    assert!(!SourceKind::CompetitorWatch.onchain_authoritative());
    assert_eq!(
        parse_stored_kind("competitor_watch"),
        SourceKind::CompetitorWatch
    );
    assert_eq!(SourceKind::CompetitorWatch.as_str(), "competitor_watch");
}

#[test]
fn t1_fixture_json_fee_and_tvl() {
    let now = chrono::Utc::now();
    let fx = vec![FixtureCompetitorPage {
        id: "defillama-terraswap".into(),
        status: 200,
        body: Some(json!({
            "name": "Terraswap",
            "fee_bps": 30,
            "currentChainTvls": { "Terra": 500000 }
        })),
        body_text: None,
        skip_robots: false,
    }];
    let watch = collect_competitor_watch_default(&fx, false, now).unwrap();
    let rec = watch
        .sources
        .iter()
        .find(|s| s.id == "competitor-watch:defillama-terraswap")
        .unwrap();
    assert_eq!(rec.kind, SourceKind::CompetitorWatch);
    assert_eq!(rec.numbers.len(), 2);
    assert!(rec.citation.starts_with("GET https://"));
}

#[test]
fn t2_null_tvl_gap_fee_stored() {
    let now = chrono::Utc::now();
    let fx = vec![FixtureCompetitorPage {
        id: "defillama-terraport".into(),
        status: 200,
        body: Some(json!({ "name": "Terraport", "fee_bps": 25, "currentChainTvls": null })),
        body_text: None,
        skip_robots: false,
    }];
    let watch = collect_competitor_watch_default(&fx, false, now).unwrap();
    let rec = watch
        .sources
        .iter()
        .find(|s| s.id.contains("terraport"))
        .unwrap();
    assert!(rec.gap.as_ref().unwrap().contains("liquidity"));
    assert!(rec.numbers.iter().any(|n| n.value == "25"));
}

#[test]
fn t4_per_page_gap_others_ok() {
    let now = chrono::Utc::now();
    let fx = vec![
        FixtureCompetitorPage {
            id: "defillama-terraswap".into(),
            status: 200,
            body: Some(json!({ "name": "T", "fee_bps": 30, "currentChainTvls": { "Terra": 1 } })),
            body_text: None,
            skip_robots: false,
        },
        FixtureCompetitorPage {
            id: "defillama-astroport-classic".into(),
            status: 503,
            body: None,
            body_text: None,
            skip_robots: false,
        },
    ];
    let watch = collect_competitor_watch_default(&fx, false, now).unwrap();
    assert_eq!(watch.sources.len(), 1);
    assert!(watch.gaps.iter().any(|g| g.contains("astroport")));
}

#[test]
fn t5_empty_page_table_gaps() {
    let watch = collect_competitor_watch(&[], &[], false, chrono::Utc::now()).unwrap();
    assert!(watch.gaps.iter().any(|g| g.contains("no competitor pages")));
}

#[test]
fn t6_watch_cli_artifacts() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("watch-out");
    let fx = vec![FixtureCompetitorPage {
        id: "defillama-terraswap".into(),
        status: 200,
        body: Some(json!({ "name": "T", "fee_bps": 30, "currentChainTvls": { "Terra": 100 } })),
        body_text: None,
        skip_robots: false,
    }];
    let watch = collect_competitor_watch_default(&fx, false, chrono::Utc::now()).unwrap();
    write_watch_artifacts(&out, &watch).unwrap();
    assert!(out.join("notes.json").exists());
    assert!(out.join("gaps.md").exists());
    assert!(!out.join("post.mdx").exists());
    assert!(!out.join("plan.json").exists());
}

#[test]
fn t7_search_finds_competitor_liquidity() {
    let now = chrono::Utc::now();
    let fx = vec![FixtureCompetitorPage {
        id: "defillama-terraswap".into(),
        status: 200,
        body: Some(json!({
            "name": "Terraswap",
            "fee_bps": 30,
            "currentChainTvls": { "Terra": 9000000 }
        })),
        body_text: None,
        skip_robots: false,
    }];
    let watch = collect_competitor_watch_default(&fx, false, now).unwrap();
    let mut store = MemoryStore::new();
    store
        .ingest_sources(&watch.sources, &HashingEmbedder)
        .unwrap();
    let hits = store
        .search(&HashingEmbedder.embed("liquidity"), 5)
        .unwrap();
    assert!(hits
        .iter()
        .any(|h| h.source_kind == SourceKind::CompetitorWatch));
}

#[test]
fn t9_competitor_volume_not_week_claim() {
    let now = chrono::Utc::now();
    let fx = vec![FixtureCompetitorPage {
        id: "defillama-terraswap".into(),
        status: 200,
        body: Some(json!({
            "name": "Terraswap",
            "fee_bps": 30,
            "currentChainTvls": { "Terra": 99999999 }
        })),
        body_text: None,
        skip_robots: false,
    }];
    let watch = collect_competitor_watch_default(&fx, false, now).unwrap();
    let mut sources = watch.sources;
    sources.push(SourceRecord {
        id: "dex-overview".into(),
        kind: SourceKind::DexIndexer,
        collected_at: now,
        citation: "GET indexer".into(),
        text: "dex volume".into(),
        numbers: vec![cl8y_research::numeric::NumericClaim {
            value: "12000".into(),
            citation: "GET indexer".into(),
            as_of: now.to_rfc3339(),
            source_id: "dex-overview".into(),
        }],
        degraded: false,
        gap: None,
        metadata: json!({}),
    });
    let claims = claims_for_week_emit(&sources);
    let body = "CL8Y DEX total_volume_24h_usd was 12000 on the indexer.";
    assert!(unsourced(body, &claims).is_empty());
    let wrong = "Their TVL was 99999999 which is not our volume.";
    assert!(unsourced(wrong, &claims).contains(&"99999999".to_string()));
}

#[test]
fn a1_ssrf_url_rejected() {
    assert!(check_competitor_watch_url("http://169.254.169.254/").is_err());
    assert!(check_competitor_watch_url("https://169.254.169.254/").is_err());
}

#[test]
fn a2_indexer_url_not_in_competitor_table() {
    assert!(check_competitor_watch_url("https://indexer.dex.cl8y.com/api/v1/overview").is_err());
}

#[test]
fn a3_lookalike_host_rejected() {
    assert!(check_competitor_watch_url("https://dex.cl8y.com.evil.example/fees").is_err());
}

#[test]
fn a5_open_redirect_query_rejected() {
    let page = COMPETITOR_WATCH_PAGES[0].url;
    assert!(check_competitor_watch_url(&format!("{page}?url=https://evil.test")).is_err());
}

#[test]
fn a11_oversize_body_gaps() {
    let page = CompetitorWatchPage {
        id: "defillama-terraswap",
        url: "https://api.llama.fi/protocol/terraswap",
        format: CompetitorPageFormat::Json,
        extractor: "defillama_protocol",
        accept: "application/json",
    };
    let huge = "x".repeat(600_000);
    let fx = FixtureCompetitorPage {
        id: "defillama-terraswap".into(),
        status: 200,
        body: Some(json!({ "name": "T", "fee_bps": 1, "note": huge })),
        body_text: None,
        skip_robots: false,
    };
    let watch = collect_competitor_watch(&[page], &[fx], false, chrono::Utc::now()).unwrap();
    assert!(watch.sources.is_empty());
    assert!(watch.gaps.iter().any(|g| g.contains("cap")));
}

#[test]
fn fixture_week_unchanged_without_competitor_pages() {
    let cfg = Config::default();
    let now = chrono::Utc.with_ymd_and_hms(2026, 8, 31, 0, 0, 0).unwrap();
    let path = PathBuf::from("fixtures/happy-week");
    let bundle = cl8y_research::collect::load_fixture_bundle(&path).unwrap();
    let c = collect_from_fixture(&cfg, &bundle, now).unwrap();
    assert!(c
        .sources
        .iter()
        .all(|s| s.kind != SourceKind::CompetitorWatch));
}
