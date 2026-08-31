//! Worker configuration. Secrets come from the environment, never from git.

use crate::error::{Error, Result};
use crate::invariants::{DEFAULT_REPOS, DEFAULT_WEEKLY_CREATE_BUDGET, EMBEDDING_DIM};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: Option<String>,
    pub replicate_token: Option<String>,
    pub telegram_bot_token: Option<String>,
    pub gitlab_token: Option<String>,
    pub github_token: Option<String>,
    pub dex_indexer_base: String,
    pub bridge_indexer_base: Option<String>,
    pub weekly_create_budget: u32,
    pub embedding_dim: usize,
    pub dry_run: bool,
    pub short_announcement: bool,
    pub skip_image: bool,
    pub team_handles: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: None,
            replicate_token: None,
            telegram_bot_token: None,
            gitlab_token: None,
            github_token: None,
            dex_indexer_base: "https://indexer.dex.cl8y.com".into(),
            bridge_indexer_base: None,
            weekly_create_budget: DEFAULT_WEEKLY_CREATE_BUDGET,
            embedding_dim: EMBEDDING_DIM,
            dry_run: false,
            short_announcement: false,
            skip_image: true,
            team_handles: vec!["PlasticDigits".into(), "ceramicliberty".into()],
        }
    }
}

impl Config {
    #[allow(clippy::field_reassign_with_default)]
    pub fn from_env() -> Result<Self> {
        let mut cfg = Self::default();
        cfg.database_url = std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());
        cfg.replicate_token = std::env::var("REPLICATE_API_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());
        cfg.telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());
        cfg.gitlab_token = std::env::var("GITLAB_TOKEN").ok().filter(|s| !s.is_empty());
        cfg.github_token = std::env::var("GITHUB_TOKEN").ok().filter(|s| !s.is_empty());
        if let Ok(v) = std::env::var("DEX_INDEXER_BASE") {
            if !v.is_empty() {
                cfg.dex_indexer_base = v.trim_end_matches('/').to_string();
            }
        }
        if let Ok(v) = std::env::var("BRIDGE_INDEXER_BASE") {
            if !v.is_empty() {
                cfg.bridge_indexer_base = Some(v.trim_end_matches('/').to_string());
            }
        }
        if let Ok(v) = std::env::var("REPLICATE_WEEKLY_CREATE_BUDGET") {
            cfg.weekly_create_budget = v
                .parse()
                .map_err(|_| Error::Config("REPLICATE_WEEKLY_CREATE_BUDGET must be u32".into()))?;
        }
        if let Ok(v) = std::env::var("TEAM_HANDLES") {
            cfg.team_handles = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        Ok(cfg)
    }

    pub fn default_repos() -> Vec<String> {
        DEFAULT_REPOS.iter().map(|s| s.to_string()).collect()
    }
}
