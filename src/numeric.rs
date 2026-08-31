//! Numeric claims must appear in `sources.json` with a timestamped citation.

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NumericClaim {
    pub value: String,
    pub citation: String,
    pub as_of: String,
    pub source_id: String,
}

pub fn extract_numbers(text: &str) -> Vec<String> {
    let re = Regex::new(r"\b\d+(?:,\d{3})*(?:\.\d+)?\b").unwrap();
    re.find_iter(text)
        .map(|m| m.as_str().replace(',', ""))
        .filter(|s| s != "0")
        .collect()
}

pub fn unsourced(body: &str, claims: &[NumericClaim]) -> Vec<String> {
    let known: Vec<String> = claims.iter().map(|c| c.value.replace(',', "")).collect();
    extract_numbers(body)
        .into_iter()
        .filter(|n| {
            // Dates like 2026 are allowed without a volume citation.
            if n.len() == 4 && n.starts_with("20") {
                return false;
            }
            // Slug-like short numerals used in headings.
            if n.len() <= 2 {
                return false;
            }
            !known.iter().any(|k| k == n)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_invented_volume() {
        let body = "Volume was 99999 as of 2026-08-31.";
        let claims = vec![NumericClaim {
            value: "12000".into(),
            citation: "GET /api/v1/overview".into(),
            as_of: "2026-08-31T00:00:00Z".into(),
            source_id: "dex-overview".into(),
        }];
        assert!(unsourced(body, &claims).contains(&"99999".to_string()));
        let ok = "Volume was 12000 as of 2026-08-31.";
        assert!(unsourced(ok, &claims).is_empty());
    }
}
