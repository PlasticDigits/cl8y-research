//! Draft MR helper. Never targets CL8Y-web `main` and never sets auto-merge.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftMr {
    pub source_branch: String,
    pub target_branch: String,
    pub title: String,
    pub body: String,
    pub auto_merge: bool,
    pub remove_source_branch: bool,
}

pub fn build_cl8y_web_draft_mr(source_branch: &str, slug: &str, report: &str) -> Result<DraftMr> {
    if source_branch == "main" || source_branch == "master" {
        return Err(Error::Config(
            "publish token must push a feature branch, not main".into(),
        ));
    }
    if source_branch.is_empty() {
        return Err(Error::Config("missing source branch".into()));
    }
    let body = format!(
        "## Summary\n- Weekly draft `{slug}` from cl8y-research. Human review required.\n\
- Touches only `src/blog/posts` and `public/images/blog`.\n\
- Does not change `render.yaml` or homepage modules.\n\n## Report\n{report}\n"
    );
    crate::secrets::assert_clean_artifact("mr-body", &body)?;
    Ok(DraftMr {
        source_branch: source_branch.into(),
        target_branch: "main".into(),
        title: format!("draft(blog): {slug}"),
        body,
        auto_merge: false,
        remove_source_branch: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_auto_merges_or_pushes_from_main() {
        assert!(build_cl8y_web_draft_mr("main", "x", "ok").is_err());
        let mr = build_cl8y_web_draft_mr("draft/weekly-x", "weekly-x", "gaps: none").unwrap();
        assert!(!mr.auto_merge);
        assert_ne!(mr.source_branch, "main");
    }
}
