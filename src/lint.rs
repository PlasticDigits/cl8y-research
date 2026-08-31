//! Mechanical editor lint. No model. Fail the run if the final MDX still trips these.

use crate::allowlist::{is_canonical_address, looks_like_address};
use crate::error::{Error, Result};
use crate::invariants::{BANNED_CURRENT_COPY, BANNED_FINANCIAL_ADVICE};
use regex::Regex;

#[derive(Debug, Clone, Default)]
pub struct LintReport {
    pub violations: Vec<String>,
}

impl LintReport {
    pub fn ok(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn fail(&self) -> Result<()> {
        if self.ok() {
            Ok(())
        } else {
            Err(Error::Lint(self.violations.join("; ")))
        }
    }
}

pub fn lint_markdown(text: &str) -> LintReport {
    let mut r = LintReport::default();
    if text.contains('\u{2014}') || text.contains('\u{2013}') {
        r.violations.push("em dash or en dash".into());
    }
    if text.contains('\u{201c}')
        || text.contains('\u{201d}')
        || text.contains('\u{2018}')
        || text.contains('\u{2019}')
    {
        r.violations.push("curly quotes".into());
    }
    let contrast = Regex::new(r"(?i)this is not [^.]+?\.\s+it is ").unwrap();
    if contrast.is_match(text) {
        r.violations.push("X is not Y. It is Z contrast".into());
    }
    let contrast2 = Regex::new(r"(?i)is not about [^.]+?\.\s+it is about ").unwrap();
    if contrast2.is_match(text) {
        r.violations
            .push("this is not about A. It is about B".into());
    }
    for phrase in BANNED_CURRENT_COPY {
        if text
            .to_ascii_lowercase()
            .contains(&phrase.to_ascii_lowercase())
        {
            r.violations
                .push(format!("banned current-marketing phrase: {phrase}"));
        }
    }
    for phrase in BANNED_FINANCIAL_ADVICE {
        if text.to_ascii_lowercase().contains(phrase) {
            r.violations
                .push(format!("financial-advice phrase: {phrase}"));
        }
    }
    let fee = Regex::new(r"(?i)\b(gold|platinum|vip|diamond)\s+tier\b").unwrap();
    if fee.is_match(text) {
        r.violations.push("invented fee-tier name".into());
    }
    let fee_pct = Regex::new(r"(?i)\b\d+(\.\d+)?%\s*(trading\s*)?fee(s)?\b").unwrap();
    if fee_pct.is_match(text) {
        r.violations.push("invented fee percent".into());
    }
    for token in text.split(|c: char| !c.is_ascii_alphanumeric()) {
        if looks_like_address(token) && !is_canonical_address(token) {
            r.violations.push(format!("unofficial address {token}"));
        }
    }
    if text.contains("<script") || text.to_ascii_lowercase().contains("javascript:") {
        r.violations.push("script or javascript: url".into());
    }
    crate::secrets::assert_clean_artifact("lint", text)
        .unwrap_or_else(|e| r.violations.push(e.to_string()));
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_tells_and_cta() {
        let dirty = "This is not X. It is Y. Buy CL8Y now — “soon”.";
        let r = lint_markdown(dirty);
        assert!(!r.ok());
        let joined = r.violations.join(" ");
        assert!(joined.contains("dash"));
        assert!(joined.contains("curly") || joined.contains("quote"));
        assert!(joined.contains("Buy CL8Y") || joined.contains("contrast"));
    }

    #[test]
    fn rejects_unofficial_address_and_advice() {
        let dirty = "Official token is 0x1111111111111111111111111111111111111111 with a risk-free return. Send funds to terra1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.";
        let r = lint_markdown(dirty);
        assert!(!r.ok());
    }

    #[test]
    fn clean_fixture_passes() {
        let clean = "CL8Y DEX listed live fee tiers on the product. Volume was 12000 as of 2026-08-31T00:00:00Z. See https://dex.cl8y.com.";
        assert!(lint_markdown(clean).ok());
    }
}
