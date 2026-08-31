//! MDX / frontmatter contract matching CL8Y-web `src/blog/blogIndex.ts`.
//!
//! Required authored fields: title, description, slug, date, author, image, tags.
//! `wordCount` is computed at **site build** time. Never author it in worker output.

use crate::allowlist::{check_post_href, is_canonical_address, looks_like_address};
use crate::error::{Error, Result};
use crate::invariants::{
    DESCRIPTION_MAX, DESCRIPTION_MIN, HERO_PATH_TEMPLATE, SLUG_PATTERN, TITLE_MAX, TITLE_MIN,
};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Frontmatter {
    pub title: String,
    pub description: String,
    pub slug: String,
    pub date: String,
    pub author: String,
    pub image: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Post {
    pub frontmatter: Frontmatter,
    pub body: String,
}

pub fn hero_path(slug: &str) -> String {
    HERO_PATH_TEMPLATE.replace("{slug}", slug)
}

pub fn parse_post(raw: &str) -> Result<Post> {
    let raw = raw.trim_start_matches('\u{feff}');
    if !raw.starts_with("---") {
        return Err(Error::Mdx("missing frontmatter fence".into()));
    }
    let rest = &raw[3..];
    let end = rest
        .find("\n---")
        .ok_or_else(|| Error::Mdx("unclosed frontmatter".into()))?;
    let yaml = &rest[..end];
    let body = rest[end + 4..].trim_start_matches('\n').to_string();
    let fm: Frontmatter =
        serde_yaml::from_str(yaml).map_err(|e| Error::Mdx(format!("frontmatter yaml: {e}")))?;
    validate_frontmatter(&fm)?;
    Ok(Post {
        frontmatter: fm,
        body,
    })
}

pub fn emit_post(post: &Post) -> Result<String> {
    validate_frontmatter(&post.frontmatter)?;
    let mut tags = String::from("tags:\n");
    for t in &post.frontmatter.tags {
        tags.push_str(&format!("  - {}\n", escape_yaml_plain(t)));
    }
    let fm = format!(
        "---\ntitle: {}\ndescription: {}\nslug: {}\ndate: {}\nauthor: {}\nimage: {}\n{tags}---\n\n",
        yaml_string(&post.frontmatter.title),
        yaml_string(&post.frontmatter.description),
        yaml_string(&post.frontmatter.slug),
        yaml_string(&post.frontmatter.date),
        yaml_string(&post.frontmatter.author),
        yaml_string(&post.frontmatter.image),
    );
    if fm.contains("wordCount") {
        return Err(Error::Mdx("wordCount must not be authored".into()));
    }
    Ok(format!("{}{}", fm, post.body.trim_start()))
}

pub fn validate_frontmatter(fm: &Frontmatter) -> Result<()> {
    for (name, value) in [
        ("title", fm.title.as_str()),
        ("description", fm.description.as_str()),
        ("slug", fm.slug.as_str()),
        ("date", fm.date.as_str()),
        ("author", fm.author.as_str()),
        ("image", fm.image.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(Error::Mdx(format!("missing frontmatter field {name}")));
        }
    }
    if fm.tags.is_empty() {
        return Err(Error::Mdx("tags must be a non-empty array".into()));
    }
    let slug_re = Regex::new(SLUG_PATTERN).unwrap();
    if !slug_re.is_match(&fm.slug) {
        return Err(Error::Mdx(format!(
            "slug {} must match {SLUG_PATTERN}",
            fm.slug
        )));
    }
    if fm.title.len() < TITLE_MIN || fm.title.len() > TITLE_MAX {
        return Err(Error::Mdx(format!(
            "title length {} out of bounds",
            fm.title.len()
        )));
    }
    if fm.description.len() < DESCRIPTION_MIN || fm.description.len() > DESCRIPTION_MAX {
        return Err(Error::Mdx(format!(
            "description length {} out of bounds",
            fm.description.len()
        )));
    }
    if fm.image != hero_path(&fm.slug) {
        return Err(Error::Mdx(format!(
            "image must be {} (got {})",
            hero_path(&fm.slug),
            fm.image
        )));
    }
    if fm.image.contains("src/blog") {
        return Err(Error::Mdx(
            "hero must not live under src/blog/assets".into(),
        ));
    }
    Ok(())
}

pub fn unique_slug(proposed: &str, existing: &[String]) -> Result<String> {
    let slug_re = Regex::new(SLUG_PATTERN).unwrap();
    if !slug_re.is_match(proposed) {
        return Err(Error::Mdx(format!("invalid slug {proposed}")));
    }
    if !existing.iter().any(|s| s == proposed) {
        return Ok(proposed.to_string());
    }
    for i in 2..50 {
        let candidate = format!("{proposed}-{i}");
        if !existing.iter().any(|s| s == &candidate) {
            return Ok(candidate);
        }
    }
    Err(Error::SlugCollision(proposed.into()))
}

pub fn sanitize_body(body: &str) -> Result<String> {
    let mut out = body.to_string();
    let lower = out.to_ascii_lowercase();
    if lower.contains("<script") || lower.contains("javascript:") {
        return Err(Error::Mdx("script / javascript: not allowed in MDX".into()));
    }
    if Regex::new(r"(?i)^\s*import\s+").unwrap().is_match(&out)
        || Regex::new(r"(?i)\nimport\s+").unwrap().is_match(&out)
    {
        return Err(Error::Mdx(
            "MDX import of runtime components is not allowed".into(),
        ));
    }
    if Regex::new(r"(?i)\son\w+\s*=").unwrap().is_match(&out) {
        return Err(Error::Mdx("HTML event handlers are not allowed".into()));
    }
    // Rewrite or drop disallowed hrefs.
    let re = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
    out = re
        .replace_all(&out, |caps: &regex::Captures| {
            let label = &caps[1];
            let href = &caps[2];
            match check_post_href(href) {
                Ok(_) => format!("[{label}]({href})"),
                Err(_) => label.to_string(),
            }
        })
        .into_owned();
    // Drop unofficial addresses rather than publish them.
    let mut cleaned = String::new();
    for token in
        out.split_inclusive(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | ')' | '('))
    {
        let core = token.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        if looks_like_address(core) && !is_canonical_address(core) {
            continue;
        }
        cleaned.push_str(token);
    }
    crate::secrets::assert_clean_artifact("mdx", &cleaned)?;
    Ok(cleaned)
}

pub fn word_count(body: &str) -> usize {
    body.split_whitespace().filter(|w| !w.is_empty()).count()
}

fn yaml_string(s: &str) -> String {
    // Quote every scalar so gray-matter keeps `date` as a string (not a Date).
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ");
    format!("\"{escaped}\"")
}

fn escape_yaml_plain(s: &str) -> String {
    if s.chars().any(|c| matches!(c, ':' | '#' | '"' | '\'')) {
        yaml_string(s)
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fm(slug: &str) -> Frontmatter {
        Frontmatter {
            title: "CL8Y DEX weekly recap for Terra Classic builders".into(),
            description: "A sourced recap of indexer volume, merged MRs, and community themes."
                .into(),
            slug: slug.into(),
            date: "2026-08-31".into(),
            author: "CL8Y Team".into(),
            image: hero_path(slug),
            tags: vec!["cl8y".into(), "dex".into()],
        }
    }

    #[test]
    fn roundtrip_omits_word_count() {
        let post = Post {
            frontmatter: sample_fm("weekly-dex-recap"),
            body: "Hello world.\n".into(),
        };
        let raw = emit_post(&post).unwrap();
        assert!(!raw.contains("wordCount"));
        assert!(raw.contains("date: \"2026-08-31\"") || raw.contains("date: \""));
        let parsed = parse_post(&raw).unwrap();
        assert_eq!(parsed.frontmatter.slug, "weekly-dex-recap");
    }

    #[test]
    fn missing_field_fails_before_publish() {
        let mut fm = sample_fm("weekly-dex-recap");
        fm.author.clear();
        assert!(validate_frontmatter(&fm).is_err());
    }

    #[test]
    fn slug_collision_suffixes() {
        let existing = vec!["cl8y-roadmap-cmm-bridge-yieldomega".into()];
        let s = unique_slug("cl8y-roadmap-cmm-bridge-yieldomega", &existing).unwrap();
        assert_eq!(s, "cl8y-roadmap-cmm-bridge-yieldomega-2");
        assert_ne!(s, existing[0]);
    }

    #[test]
    fn strips_script_and_bad_hrefs() {
        let body = "See [Bridge](https://bridge.cl8y.com.evil.example) and [ok](https://bridge.cl8y.com).\n<script>alert(1)</script>";
        assert!(sanitize_body(body).is_err());
        let ok =
            sanitize_body("See [ok](https://bridge.cl8y.com) and [bad](https://evil.test/phish).")
                .unwrap();
        assert!(ok.contains("https://bridge.cl8y.com"));
        assert!(!ok.contains("evil.test"));
    }
}
