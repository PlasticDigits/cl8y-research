//! Weekly pipeline: collect → plan → outline → draft → editor → emit.
//! Skipping a step fails the run. `--dry-run` stops after plan.

use crate::collect::{collect_from_fixture, Collection, FixtureBundle, RecentPost};
use crate::config::Config;
use crate::embed::Embedder;
use crate::error::{Error, Result};
use crate::invariants::{
    FULL_POST_WORD_MAX, FULL_POST_WORD_MIN, PIPELINE_ORDER, SHORT_POST_WORD_MAX,
};
use crate::lint::lint_markdown;
use crate::mdx::{emit_post, hero_path, sanitize_body, unique_slug, word_count, Frontmatter, Post};
use crate::numeric::{unsourced, NumericClaim};
use crate::replicate::{wrap_untrusted, PredictInput, Predictor};
use crate::secrets::assert_clean_artifact;
use crate::store::{DocumentStore, MemoryStore, SourceRecord};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Collect,
    Plan,
    Outline,
    Draft,
    Editor,
    Emit,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Collect => "collect",
            Self::Plan => "plan",
            Self::Outline => "outline",
            Self::Draft => "draft",
            Self::Editor => "editor",
            Self::Emit => "emit",
        }
    }

    fn index(self) -> usize {
        PIPELINE_ORDER
            .iter()
            .position(|s| *s == self.as_str())
            .unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub topic: String,
    pub style: String,
    pub slug: String,
    pub opening: String,
    pub short_announcement: bool,
    pub rejected_clones: Vec<String>,
    pub cited_recent: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WeekRun {
    pub completed: Vec<Stage>,
    pub collection: Option<Collection>,
    pub plan: Option<Plan>,
    pub outline: Option<String>,
    pub draft: Option<String>,
    pub passes: Vec<(String, String)>,
    pub post: Option<Post>,
    pub report: String,
    pub out_dir: PathBuf,
}

impl WeekRun {
    pub fn new(out_dir: PathBuf) -> Self {
        Self {
            completed: Vec::new(),
            collection: None,
            plan: None,
            outline: None,
            draft: None,
            passes: Vec::new(),
            post: None,
            report: String::new(),
            out_dir,
        }
    }

    fn require(&self, expected: Stage) -> Result<()> {
        // Stage i may run only when exactly i prior stages completed (collect=0).
        if self.completed.len() != expected.index() {
            return Err(Error::StageSkipped {
                expected,
                completed: self.completed.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WeekOpts {
    pub fixture_dir: PathBuf,
    pub out_dir: PathBuf,
    pub dry_run: bool,
    pub short_announcement: bool,
    pub skip_image: bool,
    pub existing_slugs: Vec<String>,
}

pub fn run_week(cfg: &Config, predictor: &mut dyn Predictor, opts: &WeekOpts) -> Result<WeekRun> {
    std::fs::create_dir_all(&opts.out_dir)?;
    let mut run = WeekRun::new(opts.out_dir.clone());
    let now = Utc::now();

    run.require(Stage::Collect)?;
    let bundle: FixtureBundle = if opts.fixture_dir.join("sources.json").exists() {
        crate::collect::load_fixture_bundle(&opts.fixture_dir)?
    } else {
        return Err(Error::Config(format!(
            "fixture sources.json missing at {}",
            opts.fixture_dir.display()
        )));
    };
    let collection = collect_from_fixture(cfg, &bundle, now)?;
    write_json(&opts.out_dir.join("sources.json"), &collection)?;
    run.collection = Some(collection.clone());
    run.completed.push(Stage::Collect);

    run.require(Stage::Plan)?;
    let plan = plan_week(
        predictor,
        &collection,
        cfg.short_announcement || opts.short_announcement,
    )?;
    write_json(&opts.out_dir.join("plan.json"), &plan)?;
    run.plan = Some(plan.clone());
    run.completed.push(Stage::Plan);

    if opts.dry_run || cfg.dry_run {
        run.report = render_report(&run, predictor, true);
        std::fs::write(opts.out_dir.join("report.md"), &run.report)?;
        return Ok(run);
    }

    run.require(Stage::Outline)?;
    let outline = step_text(
        predictor,
        "outline",
        &format!(
            "Outline a CL8Y post on {}. Follow blog_gen/SKILL.md. Sources:\n{}",
            plan.topic, collection.untrusted_prompt_block
        ),
    )?;
    std::fs::write(opts.out_dir.join("outline.md"), &outline)?;
    run.outline = Some(outline.clone());
    run.completed.push(Stage::Outline);

    run.require(Stage::Draft)?;
    let draft = step_text(
        predictor,
        "draft",
        &format!(
            "Draft the article from this outline. Voice: blog_gen/SKILL.md.\n{outline}\n{}",
            collection.untrusted_prompt_block
        ),
    )?;
    run.draft = Some(draft.clone());
    run.completed.push(Stage::Draft);

    run.require(Stage::Editor)?;
    // One predictions.create for the whole editor bundle (11 named artifacts).
    // Mechanical lint still runs at emit. Extra creates would blow the weekly budget.
    let edited = step_text(
        predictor,
        "editor",
        &format!(
            "Apply editor passes {}: grammar, punctuation, sentence-length, paragraph-rhythm, active-voice, clarity, structure, show-dont-tell, conciseness, open-close, ai-tell.\n{draft}",
            EDITOR_PASSES.join(", ")
        ),
    )?;
    std::fs::create_dir_all(opts.out_dir.join("passes"))?;
    for pass in EDITOR_PASSES {
        run.passes.push((pass.to_string(), edited.clone()));
        std::fs::write(
            opts.out_dir.join("passes").join(format!("{pass}.md")),
            &edited,
        )?;
    }
    let current = edited;
    run.completed.push(Stage::Editor);

    run.require(Stage::Emit)?;
    let slug = unique_slug(&plan.slug, &opts.existing_slugs)?;
    let sanitized = sanitize_body(&current)?;
    lint_markdown(&sanitized).fail()?;
    let claims = all_claims(&collection);
    let missing = unsourced(&sanitized, &claims);
    if !missing.is_empty() {
        return Err(Error::UnsourcedNumber {
            value: missing.join(","),
        });
    }
    let post = Post {
        frontmatter: Frontmatter {
            title: plan.topic.clone(),
            description: description_from(&sanitized, &plan),
            slug: slug.clone(),
            date: now.date_naive().to_string(),
            author: "CL8Y Team".into(),
            image: hero_path(&slug),
            tags: tags_from(&plan),
        },
        body: sanitized,
    };
    if !opts.short_announcement && !plan.short_announcement {
        let wc = word_count(&post.body);
        if wc < FULL_POST_WORD_MIN {
            return Err(Error::Mdx(format!(
                "full post has {wc} words; need {FULL_POST_WORD_MIN}-{FULL_POST_WORD_MAX} or --short"
            )));
        }
        if wc > FULL_POST_WORD_MAX + 400 {
            return Err(Error::Mdx(format!("full post too long ({wc})")));
        }
    } else {
        let wc = word_count(&post.body);
        if wc > SHORT_POST_WORD_MAX {
            return Err(Error::Mdx(format!(
                "short announcement padded to {wc} words (max {SHORT_POST_WORD_MAX})"
            )));
        }
    }
    let mdx = emit_post(&post)?;
    assert_clean_artifact("post.mdx", &mdx)?;
    std::fs::write(opts.out_dir.join("post.mdx"), &mdx)?;
    if !opts.skip_image {
        let _pred = predictor.create_once(PredictInput {
            step: "image".into(),
            model: "openai/gpt-image-2".into(),
            prompt: format!("Hero for {}", plan.topic),
        })?;
        // Image bytes are not fetched in fixture mode; path is still the contract.
    }
    write_publish_template(&opts.out_dir, &slug)?;
    run.post = Some(post);
    run.completed.push(Stage::Emit);
    run.report = render_report(&run, predictor, false);
    assert_clean_artifact("report.md", &run.report)?;
    std::fs::write(opts.out_dir.join("report.md"), &run.report)?;
    Ok(run)
}

const EDITOR_PASSES: &[&str] = &[
    "grammar",
    "punctuation",
    "sentence-length",
    "paragraph-rhythm",
    "active-voice",
    "clarity",
    "structure",
    "show-dont-tell",
    "conciseness",
    "open-close",
    "ai-tell",
];

fn step_text(predictor: &mut dyn Predictor, step: &str, prompt: &str) -> Result<String> {
    let pred = predictor.create_once(PredictInput {
        step: step.into(),
        model: "fixture-or-replicate".into(),
        prompt: wrap_untrusted("step-input", prompt),
    })?;
    let polled = predictor.poll(&pred.id)?;
    polled
        .output
        .or(pred.output)
        .ok_or_else(|| Error::Http(format!("empty output for step {step}")))
}

pub fn plan_week(
    predictor: &mut dyn Predictor,
    collection: &Collection,
    short: bool,
) -> Result<Plan> {
    let recent = &collection.recent_posts;
    let mut rejected = Vec::new();
    let proposed = step_text(
        predictor,
        "plan",
        &format!(
            "Pick a topic and style. Recent posts: {}",
            serde_json::to_string(recent).unwrap_or_default()
        ),
    )?;
    let mut plan: Plan = serde_json::from_str(&proposed).unwrap_or_else(|_| Plan {
        topic: proposed
            .lines()
            .next()
            .unwrap_or("Weekly CL8Y ecosystem recap")
            .to_string(),
        style: "field-note".into(),
        slug: "weekly-cl8y-ecosystem-recap".into(),
        opening: proposed
            .lines()
            .nth(1)
            .unwrap_or("This week the stack moved in public.")
            .to_string(),
        short_announcement: short,
        rejected_clones: Vec::new(),
        cited_recent: recent.iter().map(|p| p.slug.clone()).collect(),
    });
    plan.cited_recent = recent.iter().map(|p| p.slug.clone()).collect();
    plan.short_announcement = short || plan.short_announcement;
    if let Some(why) = diversity_reject(&plan, recent) {
        rejected.push(why.clone());
        plan.rejected_clones = rejected;
        return Err(Error::Diversity(why));
    }
    plan.rejected_clones = rejected;
    if plan.topic.len() < 12 {
        return Err(Error::Diversity("topic too thin".into()));
    }
    Ok(plan)
}

pub fn diversity_reject(plan: &Plan, recent: &[RecentPost]) -> Option<String> {
    for post in recent {
        if jaccard(&plan.topic, &post.title) > 0.72 {
            return Some(format!("title too close to {}", post.slug));
        }
        if !plan.opening.is_empty() && plan.opening.trim() == post.opening.trim() {
            return Some(format!("opening clones {}", post.slug));
        }
        let a: std::collections::BTreeSet<_> = tags_from(plan).into_iter().collect();
        let b: std::collections::BTreeSet<_> = post.tags.iter().cloned().collect();
        if !a.is_empty() && a == b {
            return Some(format!("tag set clones {}", post.slug));
        }
    }
    None
}

fn jaccard(a: &str, b: &str) -> f32 {
    let ta: std::collections::BTreeSet<_> = crate::embed::tokenize(a).into_iter().collect();
    let tb: std::collections::BTreeSet<_> = crate::embed::tokenize(b).into_iter().collect();
    if ta.is_empty() || tb.is_empty() {
        return 0.0;
    }
    let inter = ta.intersection(&tb).count() as f32;
    let union = ta.union(&tb).count() as f32;
    inter / union
}

fn tags_from(plan: &Plan) -> Vec<String> {
    let mut tags = vec!["cl8y".into()];
    let t = plan.topic.to_ascii_lowercase();
    if t.contains("bridge") {
        tags.push("bridge".into());
    }
    if t.contains("dex") {
        tags.push("dex".into());
    }
    if t.contains("yield") {
        tags.push("yieldomega".into());
    }
    tags.sort();
    tags.dedup();
    tags
}

fn description_from(body: &str, plan: &Plan) -> String {
    let first = body
        .lines()
        .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .unwrap_or(&plan.opening)
        .trim()
        .to_string();
    let mut d = first;
    if d.len() < 24 {
        d = format!("{} {}", d, plan.topic);
    }
    if d.len() > 240 {
        d.truncate(237);
        d.push_str("...");
    }
    d
}

fn all_claims(c: &Collection) -> Vec<NumericClaim> {
    c.sources.iter().flat_map(|s| s.numbers.clone()).collect()
}

fn write_json<T: Serialize>(path: &Path, v: &T) -> Result<()> {
    std::fs::write(path, serde_json::to_string_pretty(v)?)?;
    Ok(())
}

fn write_publish_template(out: &Path, slug: &str) -> Result<()> {
    let body = format!(
        "# Draft publish for CL8Y-web\n\n\
This worker does **not** push to `CL8Y-web` `main` and does **not** auto-merge.\n\n\
Copy only:\n\
- `post.mdx` → `src/blog/posts/{slug}.mdx`\n\
- hero → `public/images/blog/{slug}-hero.jpg`\n\n\
Open a human-reviewed MR. Token must be contents:write on a feature branch only.\n"
    );
    std::fs::write(out.join("publish-mr.md"), body)?;
    Ok(())
}

fn render_report(run: &WeekRun, predictor: &dyn Predictor, dry: bool) -> String {
    let gaps = run
        .collection
        .as_ref()
        .map(|c| c.gaps.join("\n- "))
        .unwrap_or_default();
    format!(
        "# Week report\n\nDry run: {dry}\n\nStages: {:?}\n\nCreates: {}\nIds: {:?}\n\nGaps:\n- {gaps}\n\nPlan: {}\n",
        run.completed.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        predictor.create_count(),
        predictor.created_ids(),
        run.plan.as_ref().map(|p| p.topic.as_str()).unwrap_or("(none)"),
    )
}

/// Used by tests to prove emit-without-editor fails closed.
pub fn emit_without_prior_stages(run: &WeekRun) -> Result<()> {
    run.require(Stage::Emit)
}

pub fn search_collection(
    collection: &Collection,
    query: &str,
    k: usize,
) -> Result<Vec<crate::store::SearchHit>> {
    let mut store = MemoryStore::new();
    let e = crate::store::default_embedder();
    store.ingest_sources(&collection.sources, &e)?;
    store.search(&e.embed(query), k)
}

pub fn sources_as_records(c: &Collection) -> &[SourceRecord] {
    &c.sources
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replicate::FixturePredictor;
    use std::collections::HashMap;

    fn sample_body() -> String {
        let claim =
            "Indexer overview reported 12000 USD trailing volume as of 2026-08-31T00:00:00Z.";
        let para = format!(
            "{claim} The DEX HTTP API is the source. Community themes stayed on fee-tier curiosity without quoting wallets. Merged work in yieldomega stayed documentation shaped.\n\n"
        );
        let mut body = String::from("## What moved\n\n");
        while word_count(&body) < FULL_POST_WORD_MIN {
            body.push_str(&para);
        }
        body.push_str("\n## Close\n\nThe week is public. Code and issues remain on GitLab.\n");
        body
    }

    fn outputs() -> HashMap<String, String> {
        let plan = serde_json::json!({
            "topic": "CL8Y DEX volume and public indexer work this week",
            "style": "field-note",
            "slug": "weekly-dex-indexer-volume",
            "opening": "The indexer told a quieter story than last month's roadmap.",
            "short_announcement": false,
            "rejected_clones": [],
            "cited_recent": ["cl8y-roadmap-cmm-bridge-yieldomega"]
        })
        .to_string();
        let body = sample_body();
        let mut m = HashMap::new();
        m.insert("plan".into(), plan);
        m.insert("outline".into(), "## Volume\n## Repos\n## Close\n".into());
        m.insert("draft".into(), body.clone());
        m.insert("editor".into(), body);
        m
    }

    fn bundle() -> FixtureBundle {
        FixtureBundle {
            repos: vec![crate::collect::FixtureRepo {
                remote: "github.com/PlasticDigits/yieldomega".into(),
                events: vec!["merged MR 12: docs".into()],
                execute_me: None,
            }],
            dex_overview: Some(serde_json::json!({"total_volume_24h_usd": "12000"})),
            dex_error: None,
            bridge_overview: None,
            bridge_error: None,
            telegram: vec![crate::telegram::TelegramMessage {
                room: "ceramicliberty".into(),
                message_id: "9".into(),
                from: "PlasticDigits".into(),
                text: "@PlasticDigits fee tiers live on the DEX".into(),
                is_team: true,
            }],
            recent_posts: vec![RecentPost {
                slug: "cl8y-roadmap-cmm-bridge-yieldomega".into(),
                title: "The Updated CL8Y Roadmap for CMM Treasury, Bridge Liquidity, YieldOmega, and UST1".into(),
                tags: vec!["roadmap".into(), "ust1".into()],
                opening: "CL8Y has grown into a set of connected infrastructure".into(),
            }],
            ssrf_url: None,
        }
    }

    #[test]
    fn skipping_emit_fails() {
        let run = WeekRun::new(PathBuf::from("/tmp"));
        assert!(matches!(
            emit_without_prior_stages(&run),
            Err(Error::StageSkipped {
                expected: Stage::Emit,
                ..
            })
        ));
    }

    #[test]
    fn diversity_rejects_clone_opening() {
        let plan = Plan {
            topic: "Bridge volume recap".into(),
            style: "recap".into(),
            slug: "bridge-volume-recap".into(),
            opening: "Same opening sentence".into(),
            short_announcement: false,
            rejected_clones: vec![],
            cited_recent: vec![],
        };
        let recent = vec![RecentPost {
            slug: "last-week".into(),
            title: "Bridge volume recap".into(),
            tags: vec!["bridge".into()],
            opening: "Same opening sentence".into(),
        }];
        assert!(diversity_reject(&plan, &recent).is_some());
    }

    #[test]
    fn happy_path_writes_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let fixtures = dir.path().join("fx");
        std::fs::create_dir_all(&fixtures).unwrap();
        std::fs::write(
            fixtures.join("sources.json"),
            serde_json::to_vec(&bundle()).unwrap(),
        )
        .unwrap();
        let out = dir.path().join("out");
        let mut pred = FixturePredictor::new(outputs());
        let cfg = Config::default();
        let run = run_week(
            &cfg,
            &mut pred,
            &WeekOpts {
                fixture_dir: fixtures,
                out_dir: out.clone(),
                dry_run: false,
                short_announcement: false,
                skip_image: true,
                existing_slugs: vec!["cl8y-roadmap-cmm-bridge-yieldomega".into()],
            },
        )
        .unwrap();
        assert!(out.join("plan.json").exists());
        assert!(out.join("outline.md").exists());
        assert!(out.join("post.mdx").exists());
        assert!(out.join("report.md").exists());
        assert!(out.join("passes").join("grammar.md").exists());
        let mdx = std::fs::read_to_string(out.join("post.mdx")).unwrap();
        assert!(!mdx.contains("wordCount"));
        assert!(
            mdx.contains("image: /images/blog/weekly-dex-indexer-volume-hero.jpg")
                || mdx.contains("/images/blog/")
        );
        assert_eq!(run.completed.last().copied(), Some(Stage::Emit));
        assert!(pred.create_count() >= 1);
    }

    #[test]
    fn dry_run_stops_after_plan() {
        let dir = tempfile::tempdir().unwrap();
        let fixtures = dir.path().join("fx");
        std::fs::create_dir_all(&fixtures).unwrap();
        std::fs::write(
            fixtures.join("sources.json"),
            serde_json::to_vec(&bundle()).unwrap(),
        )
        .unwrap();
        let mut pred = FixturePredictor::new(outputs());
        let cfg = Config {
            dry_run: true,
            ..Config::default()
        };
        let run = run_week(
            &cfg,
            &mut pred,
            &WeekOpts {
                fixture_dir: fixtures,
                out_dir: dir.path().join("out"),
                dry_run: true,
                short_announcement: false,
                skip_image: true,
                existing_slugs: vec![],
            },
        )
        .unwrap();
        assert_eq!(run.completed, vec![Stage::Collect, Stage::Plan]);
        assert!(run.post.is_none());
    }
}
