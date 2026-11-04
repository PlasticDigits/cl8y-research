//! cl8y-research CLI.

use cl8y_research::collect::load_fixture_bundle;
use cl8y_research::competitor_watch::{
    collect_competitor_watch_default, load_fixture_competitor_pages, load_watch_notes,
    write_watch_artifacts,
};
use cl8y_research::config::Config;
use cl8y_research::embed::{Embedder, HashingEmbedder};
use cl8y_research::pipeline::{run_week, WeekOpts};
use cl8y_research::replicate::{FixturePredictor, HttpPredictor, Predictor};
use cl8y_research::store::{DocumentStore, MemoryStore, SourceRecord};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "cl8y-research",
    about = "CL8Y weekly blog worker and vector search"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Collect → plan → outline → draft → editor → emit. `--dry-run` stops after plan.
    Week {
        #[arg(long, default_value = "fixtures/happy-week")]
        fixtures: PathBuf,
        #[arg(long, default_value = "runs/latest")]
        out: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        short: bool,
        #[arg(long, default_value_t = true)]
        skip_image: bool,
        #[arg(long)]
        live: bool,
    },
    /// Ingest fixture sources into the in-memory (or Postgres) store.
    Ingest {
        #[arg(long, default_value = "fixtures/happy-week")]
        fixtures: PathBuf,
    },
    /// Vector search over ingested sources (no live credentials required for fixtures).
    Search {
        query: String,
        #[arg(long, default_value = "fixtures/happy-week")]
        fixtures: PathBuf,
        #[arg(long)]
        from: Option<PathBuf>,
        #[arg(long, default_value_t = 8)]
        k: usize,
    },
    /// Competitor fee/liquidity watch → raw notes inbox (not a week pipeline run).
    Watch {
        #[arg(long, default_value = "fixtures/competitor-watch")]
        fixtures: PathBuf,
        #[arg(long, default_value = "runs/watch/latest")]
        out: PathBuf,
        #[arg(long)]
        live_watch: bool,
    },
    /// Recover a Replicate prediction by id (no second create).
    Fetch { id: String },
    /// Apply Postgres migrations (requires `--features postgres` and DATABASE_URL).
    Migrate,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> cl8y_research::Result<()> {
    let cli = Cli::parse();
    let mut cfg = Config::from_env()?;
    match cli.cmd {
        Cmd::Week {
            fixtures,
            out,
            dry_run,
            short,
            skip_image,
            live,
        } => {
            cfg.dry_run = dry_run;
            cfg.short_announcement = short;
            cfg.skip_image = skip_image;
            let mut predictor: Box<dyn Predictor> = if live {
                let token = cfg.replicate_token.clone().ok_or_else(|| {
                    cl8y_research::Error::Config("REPLICATE_API_TOKEN required for --live".into())
                })?;
                Box::new(HttpPredictor::new(token, cfg.weekly_create_budget)?)
            } else {
                Box::new(load_fixture_predictor(&fixtures)?)
            };
            let existing = load_existing_slugs(&fixtures);
            let run = run_week(
                &cfg,
                predictor.as_mut(),
                &WeekOpts {
                    fixture_dir: fixtures,
                    out_dir: out.clone(),
                    dry_run,
                    short_announcement: short,
                    skip_image,
                    existing_slugs: existing,
                },
            )?;
            println!("completed {:?} → {}", run.completed, out.display());
            Ok(())
        }
        Cmd::Ingest { fixtures } => {
            let sources = load_search_sources(&fixtures, None)?;
            let mut store = MemoryStore::new();
            let n = store.ingest_sources(&sources, &HashingEmbedder)?;
            println!("ingested {n} documents (in-memory). Use `search` to query.");
            Ok(())
        }
        Cmd::Search {
            query,
            fixtures,
            from,
            k,
        } => {
            let sources = load_search_sources(&fixtures, from.as_deref())?;
            let mut store = MemoryStore::new();
            store.ingest_sources(&sources, &HashingEmbedder)?;
            let hits = store.search(&HashingEmbedder.embed(&query), k)?;
            for h in hits {
                println!(
                    "{:.3} {} {}\n  {}\n",
                    h.score,
                    h.source_kind.as_str(),
                    h.source_id,
                    h.citation
                );
            }
            Ok(())
        }
        Cmd::Watch {
            fixtures,
            out,
            live_watch,
        } => {
            let now = chrono::Utc::now();
            let bundle = load_fixture_bundle(&fixtures).ok();
            let fx = bundle
                .map(|b| b.competitor_watch)
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| load_fixture_competitor_pages(&fixtures).unwrap_or_default());
            let watch = collect_competitor_watch_default(&fx, live_watch, now)?;
            write_watch_artifacts(&out, &watch)?;
            let mut store = MemoryStore::new();
            let n = store.ingest_sources(&watch.sources, &HashingEmbedder)?;
            println!(
                "watch: {} notes, {} gaps → {}",
                n,
                watch.gaps.len(),
                out.display()
            );
            Ok(())
        }
        Cmd::Fetch { id } => {
            let token = cfg.replicate_token.clone().ok_or_else(|| {
                cl8y_research::Error::Config("REPLICATE_API_TOKEN required for fetch".into())
            })?;
            let mut http = HttpPredictor::new(token, cfg.weekly_create_budget)?;
            let p = http.fetch(&id)?;
            println!("{} {}", p.id, p.status);
            Ok(())
        }
        Cmd::Migrate => {
            #[cfg(feature = "postgres")]
            {
                let url = cfg
                    .database_url
                    .clone()
                    .ok_or_else(|| cl8y_research::Error::Config("DATABASE_URL required".into()))?;
                let rt = tokio::runtime::Runtime::new()
                    .map_err(|e| cl8y_research::Error::Other(e.to_string()))?;
                rt.block_on(async {
                    let pool = cl8y_research::store::postgres::connect(&url).await?;
                    cl8y_research::store::postgres::migrate(&pool).await
                })
            }
            #[cfg(not(feature = "postgres"))]
            {
                Err(cl8y_research::Error::Config(
                    "rebuild with --features postgres".into(),
                ))
            }
        }
    }
}

fn load_fixture_predictor(dir: &std::path::Path) -> cl8y_research::Result<FixturePredictor> {
    let path = dir.join("replicate.json");
    let map: HashMap<String, String> = if path.exists() {
        serde_json::from_str(&std::fs::read_to_string(path)?)?
    } else {
        HashMap::new()
    };
    Ok(FixturePredictor::new(map))
}

fn load_search_sources(
    fixtures: &std::path::Path,
    from_watch: Option<&std::path::Path>,
) -> cl8y_research::Result<Vec<SourceRecord>> {
    let mut sources = Vec::new();
    if let Some(dir) = from_watch {
        let notes = dir.join("notes.json");
        if notes.exists() {
            sources.extend(load_watch_notes(&notes)?);
        }
    }
    if fixtures.join("sources.json").exists() {
        let bundle = load_fixture_bundle(fixtures)?;
        if !bundle.competitor_watch.is_empty() {
            let now = chrono::Utc::now();
            let watch = collect_competitor_watch_default(&bundle.competitor_watch, false, now)?;
            sources.extend(watch.sources);
        } else if fixtures.to_string_lossy().contains("competitor-watch") {
            let fx = load_fixture_competitor_pages(fixtures)?;
            let now = chrono::Utc::now();
            let watch = collect_competitor_watch_default(&fx, false, now)?;
            sources.extend(watch.sources);
        } else {
            let collection = cl8y_research::collect::collect_from_fixture(
                &Config::from_env()?,
                &bundle,
                chrono::Utc::now(),
            )?;
            sources.extend(collection.sources);
        }
    }
    Ok(sources)
}

fn load_existing_slugs(dir: &std::path::Path) -> Vec<String> {
    let path = dir.join("existing_slugs.json");
    if let Ok(raw) = std::fs::read_to_string(path) {
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        vec!["cl8y-roadmap-cmm-bridge-yieldomega".into()]
    }
}
