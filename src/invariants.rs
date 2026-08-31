//! Typed invariants for cl8y-research ([`cl8y-research#1`](https://gitlab.com/PlasticDigits/cl8y-research/-/issues/1)).
//!
//! These constants are the source of truth that tests and docs must stay
//! aligned with. Cross-links:
//! - [`docs/invariants.md`](../docs/invariants.md)
//! - [`skills/cl8y-research-worker/SKILL.md`](../skills/cl8y-research-worker/SKILL.md)
//! - [`skills/cl8y-research-search/SKILL.md`](../skills/cl8y-research-search/SKILL.md)
//! - Voice contract: `CL8Y-web` [`blog_gen/SKILL.md`](https://gitlab.com/PlasticDigits/CL8Y-web/-/blob/main/blog_gen/SKILL.md)
//! - Addresses: `CL8Y-web` GitLab #2 / [`src/data/tokenDirectory.ts`](https://gitlab.com/PlasticDigits/CL8Y-web/-/blob/main/src/data/tokenDirectory.ts)

/// Worker lives here so CL8Y-web stays a static Vite SPA.
pub const REPO_PLACEMENT: &str = "dedicated-repo-cl8y-research";

/// Pipeline order. Skipping a step fails the run.
pub const PIPELINE_ORDER: &[&str] = &["collect", "plan", "outline", "draft", "editor", "emit"];

/// Full posts target this range unless the short-announcement flag is set.
pub const FULL_POST_WORD_MIN: usize = 2000;
pub const FULL_POST_WORD_MAX: usize = 2500;

/// Short announcement upper bound. Do not pad with generic DeFi filler.
pub const SHORT_POST_WORD_MAX: usize = 800;

pub const SLUG_PATTERN: &str = r"^[a-z0-9-]+$";

/// Public hero path template. Never `src/blog/assets`.
pub const HERO_PATH_TEMPLATE: &str = "/images/blog/{slug}-hero.jpg";

/// Default weekly Replicate `predictions.create` cap (plan, outline, draft, image).
pub const DEFAULT_WEEKLY_CREATE_BUDGET: u32 = 6;

/// Telegram retention window in days. Full room history is not kept longer.
pub const TELEGRAM_RETENTION_DAYS: i64 = 14;

/// Official rooms only. Do not scrape public web previews.
pub const TELEGRAM_ROOMS: &[&str] = &["plasticann", "ceramicliberty"];

/// Official CL8Y token addresses (CL8Y-web #2). Unofficial addresses must not appear in MDX.
pub const CANONICAL_ADDRESSES: &[&str] = &[
    "0x8F452a1fdd388A45e1080992eFF051b4dd9048d2",
    "terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3",
    "0xfBAa45A537cF07dC768c469FfaC4e88208B0098D",
    "0xBe9F06b76e301b49Dc345948a7a5E3418264886A",
];

/// Hashing embedder dimension. Live Replicate embeddings must match `EMBEDDING_DIM`.
pub const EMBEDDING_DIM: usize = 64;

/// Title / description bounds (SEO / spam).
pub const TITLE_MIN: usize = 12;
pub const TITLE_MAX: usize = 140;
pub const DESCRIPTION_MIN: usize = 24;
pub const DESCRIPTION_MAX: usize = 240;

/// Banned current-marketing phrases (homepage copy, not ecosystem history).
pub const BANNED_CURRENT_COPY: &[&str] = &[
    "Buy CL8Y",
    "the future of DeFi",
    "Autoscarcity",
    "expensive token",
    "GameFi",
    "PROTOCASS",
    "Karnyx",
    "TigerHunt",
    "AscendEX",
];

/// Return-promise / deposit-instruction phrases stripped by lint.
pub const BANNED_FINANCIAL_ADVICE: &[&str] = &[
    "risk-free return",
    "risk free return",
    "guaranteed return",
    "send funds to",
    "send crypto to",
    "deposit to this address",
];

/// Product href hosts allowed in generated MDX.
pub const POST_HREF_HOSTS: &[&str] = &[
    "cl8y.com",
    "www.cl8y.com",
    "bridge.cl8y.com",
    "dex.cl8y.com",
    "ust1cmm.com",
    "gitlab.com",
    "github.com",
    "t.me",
    "indexer.dex.cl8y.com",
];

/// HTTP GET allowlist for collectors (no HTML scrape of dapp UIs).
pub const INDEXER_HOSTS: &[&str] = &["indexer.dex.cl8y.com", "indexer.bridge.cl8y.com"];

/// Git hosts whose org path must be PlasticDigits, CeramicLiberty, or CL8Y.
pub const REPO_GIT_HOSTS: &[&str] = &["github.com", "gitlab.com"];
pub const REPO_ORGS: &[&str] = &["PlasticDigits", "CeramicLiberty", "CL8Y"];

/// Default allowlisted remotes from blog_gen/SKILL.md.
pub const DEFAULT_REPOS: &[&str] = &[
    "github.com/PlasticDigits/yieldomega",
    "github.com/PlasticDigits/cl8y-bridge-monorepo",
    "github.com/PlasticDigits/cl8y-dex-terraclassic",
    "github.com/PlasticDigits/ustr-cmm",
    "gitlab.com/PlasticDigits/CL8Y-web",
    "gitlab.com/PlasticDigits/cl8y-research",
];

/// DEX overview path (documented in cl8y-dex-terraclassic indexer).
pub const DEX_OVERVIEW_PATH: &str = "/api/v1/overview";

/// Bridge indexer public HTTP API is not documented in cl8y-bridge-monorepo
/// as of this worker. Collectors must skip the stat rather than invent an API.
pub const BRIDGE_INDEXER_GAP: &str =
    "No documented public bridge-indexer HTTP API in cl8y-bridge-monorepo; skip volume rather than invent an endpoint.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_order_is_six_steps() {
        assert_eq!(
            PIPELINE_ORDER,
            &["collect", "plan", "outline", "draft", "editor", "emit"]
        );
    }

    #[test]
    fn hero_path_never_uses_src_blog_assets() {
        assert!(!HERO_PATH_TEMPLATE.contains("src/blog"));
        assert!(HERO_PATH_TEMPLATE.starts_with("/images/blog/"));
    }
}
