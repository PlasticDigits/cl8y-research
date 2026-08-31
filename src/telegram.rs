//! Telegram handling: official Bot API as a room member, never web scrape.
//!
//! Chat is theme and sentiment only. It is never an onchain source of truth.
//! Non-team identities are redacted. Retention is [`crate::invariants::TELEGRAM_RETENTION_DAYS`].

use crate::allowlist::{is_canonical_address, looks_like_address};
use crate::invariants::TELEGRAM_ROOMS;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramMessage {
    pub room: String,
    pub message_id: String,
    pub from: String,
    pub text: String,
    pub is_team: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramAggregate {
    pub room: String,
    pub themes: Vec<String>,
    pub sentiment: String,
    pub team_announcements: Vec<String>,
    pub dropped_threads: usize,
    pub redacted_identities: usize,
}

pub fn assert_allowlisted_room(room: &str) -> crate::error::Result<()> {
    let r = room
        .trim()
        .trim_start_matches('@')
        .trim_start_matches("https://t.me/")
        .trim_start_matches("http://t.me/")
        .trim_start_matches("t.me/");
    if !TELEGRAM_ROOMS.iter().any(|x| x.eq_ignore_ascii_case(r)) {
        return Err(crate::error::Error::Allowlist(format!(
            "telegram room {room} is not allowlisted"
        )));
    }
    Ok(())
}

pub fn redact_message(msg: &TelegramMessage, team_handles: &[String]) -> (String, bool) {
    let mut redacted_any = false;
    let mut text = msg.text.clone();
    let handle_re = Regex::new(r"@[A-Za-z0-9_]{3,32}").unwrap();
    text = handle_re
        .replace_all(&text, |caps: &regex::Captures| {
            let h = caps[0].trim_start_matches('@');
            if team_handles.iter().any(|t| t.eq_ignore_ascii_case(h)) {
                caps[0].to_string()
            } else {
                redacted_any = true;
                "[redacted]".into()
            }
        })
        .into_owned();
    let mut cleaned = String::new();
    for token in text.split_inclusive(char::is_whitespace) {
        let core = token.trim();
        if looks_like_address(core) && !is_canonical_address(core) {
            redacted_any = true;
            continue;
        }
        cleaned.push_str(token);
    }
    // Do not keep raw non-team quotes.
    if !msg.is_team {
        let lowered = cleaned.to_ascii_lowercase();
        if lowered.contains("insider") || lowered.contains("scam") || lowered.contains("liar") {
            return (String::new(), true);
        }
    }
    (cleaned, redacted_any)
}

pub fn aggregate(messages: &[TelegramMessage], team_handles: &[String]) -> Vec<TelegramAggregate> {
    let mut by_room: std::collections::BTreeMap<String, Vec<(String, bool, bool)>> =
        std::collections::BTreeMap::new();
    for msg in messages {
        if assert_allowlisted_room(&msg.room).is_err() {
            continue;
        }
        let (text, redacted) = redact_message(msg, team_handles);
        by_room
            .entry(msg.room.clone())
            .or_default()
            .push((text, msg.is_team, redacted));
    }
    by_room
        .into_iter()
        .map(|(room, rows)| {
            let mut themes = Vec::new();
            let mut announcements = Vec::new();
            let mut dropped = 0;
            let mut redacted_identities = 0;
            for (text, is_team, redacted) in &rows {
                if *redacted {
                    redacted_identities += 1;
                }
                if text.is_empty() {
                    dropped += 1;
                    continue;
                }
                if *is_team {
                    announcements.push(summarize_theme(text));
                } else {
                    themes.push(summarize_theme(text));
                }
            }
            themes.sort();
            themes.dedup();
            let sentiment = if announcements.is_empty() && themes.is_empty() {
                "quiet".into()
            } else {
                "active".into()
            };
            TelegramAggregate {
                room,
                themes,
                sentiment,
                team_announcements: announcements,
                dropped_threads: dropped,
                redacted_identities,
            }
        })
        .collect()
}

fn summarize_theme(text: &str) -> String {
    let t = text
        .split_whitespace()
        .take(12)
        .collect::<Vec<_>>()
        .join(" ");
    t.chars().take(120).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_handles_wallets_and_drops_attacks() {
        let team = vec!["PlasticDigits".into()];
        let msgs = vec![
            TelegramMessage {
                room: "ceramicliberty".into(),
                message_id: "1".into(),
                from: "alice".into(),
                text: "hey @randomuser check 0x2222222222222222222222222222222222222222".into(),
                is_team: false,
            },
            TelegramMessage {
                room: "plasticann".into(),
                message_id: "2".into(),
                from: "PlasticDigits".into(),
                text: "@PlasticDigits shipped the indexer overview endpoint".into(),
                is_team: true,
            },
            TelegramMessage {
                room: "ceramicliberty".into(),
                message_id: "3".into(),
                from: "mallory".into(),
                text: "insider leak: Bob is a liar".into(),
                is_team: false,
            },
        ];
        let agg = aggregate(&msgs, &team);
        let joined = serde_json::to_string(&agg).unwrap();
        assert!(!joined.contains("randomuser"));
        assert!(!joined.contains("0x2222"));
        assert!(!joined.contains("Bob is a liar"));
        assert!(joined.contains("indexer overview") || joined.to_lowercase().contains("shipped"));
        assert!(joined.contains("ceramicliberty"));
    }

    #[test]
    fn rejects_other_rooms() {
        assert!(assert_allowlisted_room("randomchan").is_err());
        assert!(assert_allowlisted_room("t.me/ceramicliberty").is_ok());
    }
}
