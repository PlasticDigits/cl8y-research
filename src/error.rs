//! Error types for the research worker.
//!
//! Fail closed on skipped pipeline stages, budget exhaustion, and lint
//! violations. Never retry `predictions.create`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("pipeline skipped required stage {expected:?} (completed: {completed:?})")]
    StageSkipped {
        expected: crate::pipeline::Stage,
        completed: Vec<crate::pipeline::Stage>,
    },
    #[error("replicate create already used for this step; recover with prediction id {0}")]
    CreateOnce(String),
    #[error("replicate weekly budget exhausted ({used}/{max})")]
    BudgetExhausted { used: u32, max: u32 },
    #[error("mechanical lint failed: {0}")]
    Lint(String),
    #[error("mdx contract failed: {0}")]
    Mdx(String),
    #[error("url not allowlisted: {0}")]
    Allowlist(String),
    #[error("ssrf / blocked host: {0}")]
    Ssrf(String),
    #[error("topic diversity: {0}")]
    Diversity(String),
    #[error("slug collision for {0}; refusing to overwrite")]
    SlugCollision(String),
    #[error("numeric claim {value:?} missing from sources.json")]
    UnsourcedNumber { value: String },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("http: {0}")]
    Http(String),
    #[error("config: {0}")]
    Config(String),
    #[error("store: {0}")]
    Store(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
