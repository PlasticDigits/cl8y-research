//! Secret redaction. Artifacts, logs, and MR bodies must never contain tokens.

const PATTERNS: &[&str] = &[
    "REPLICATE_",
    "BOT_TOKEN",
    "api_key",
    "API_KEY",
    "BEGIN ",
    "TELEGRAM_SESSION",
    "GITLAB_TOKEN",
    "GITHUB_TOKEN",
    "DATABASE_URL",
];

pub fn contains_secret_marker(text: &str) -> bool {
    PATTERNS.iter().any(|p| text.contains(p))
}

pub fn redact(text: &str) -> String {
    let mut out = text.to_string();
    for needle in [
        "REPLICATE_API_TOKEN",
        "BOT_TOKEN",
        "TELEGRAM_API_HASH",
        "TELEGRAM_API_ID",
        "GITLAB_TOKEN",
        "GITHUB_TOKEN",
        "DATABASE_URL",
        "INDEXER_API_KEY",
    ] {
        if let Ok(re) = regex::Regex::new(&format!(r"(?i){needle}\s*[=:]\s*\S+")) {
            out = re
                .replace_all(&out, format!("{needle}=[redacted]"))
                .into_owned();
        }
    }
    out
}

pub fn assert_clean_artifact(label: &str, text: &str) -> crate::error::Result<()> {
    if contains_secret_marker(text) {
        return Err(crate::error::Error::Lint(format!(
            "{label} contains a secret marker"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catches_token_echo() {
        assert!(contains_secret_marker("export REPLICATE_API_TOKEN=abc"));
        assert!(contains_secret_marker("BOT_TOKEN=123"));
        assert!(contains_secret_marker("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(!contains_secret_marker("weekly volume was 12"));
    }
}
