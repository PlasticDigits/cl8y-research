//! Collected primary sources and persistence for vector search.

use crate::embed::{cosine, Embedder, HashingEmbedder};
#[cfg(feature = "postgres")]
use crate::error::Error;
use crate::error::Result;
use crate::numeric::NumericClaim;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Repo,
    DexIndexer,
    BridgeIndexer,
    Telegram,
    RecentPost,
    CompetitorWatch,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Repo => "repo",
            Self::DexIndexer => "dex_indexer",
            Self::BridgeIndexer => "bridge_indexer",
            Self::Telegram => "telegram",
            Self::RecentPost => "recent_post",
            Self::CompetitorWatch => "competitor_watch",
        }
    }

    /// Telegram and competitor pages are never onchain sources of truth.
    pub fn onchain_authoritative(self) -> bool {
        matches!(self, Self::DexIndexer | Self::BridgeIndexer | Self::Repo)
    }
}

/// Postgres round-trip for `source_kind` text (cl8y-research#14).
pub fn parse_stored_kind(s: &str) -> SourceKind {
    match s {
        "dex_indexer" => SourceKind::DexIndexer,
        "bridge_indexer" => SourceKind::BridgeIndexer,
        "telegram" => SourceKind::Telegram,
        "recent_post" => SourceKind::RecentPost,
        "competitor_watch" => SourceKind::CompetitorWatch,
        "repo" => SourceKind::Repo,
        _ => SourceKind::Repo,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRecord {
    pub id: String,
    pub kind: SourceKind,
    pub collected_at: DateTime<Utc>,
    pub citation: String,
    pub text: String,
    pub numbers: Vec<NumericClaim>,
    pub degraded: bool,
    pub gap: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub source_kind: SourceKind,
    pub source_id: String,
    pub collected_at: DateTime<Utc>,
    pub citation: String,
    pub body: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub source_id: String,
    pub source_kind: SourceKind,
    pub citation: String,
    pub body: String,
    pub score: f32,
}

pub trait DocumentStore {
    fn upsert(&mut self, doc: Document) -> Result<()>;
    fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchHit>>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Default)]
pub struct MemoryStore {
    docs: Vec<Document>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self { docs: Vec::new() }
    }

    pub fn ingest_sources(
        &mut self,
        sources: &[SourceRecord],
        embedder: &dyn Embedder,
    ) -> Result<usize> {
        let mut n = 0;
        for s in sources {
            if s.text.trim().is_empty() {
                continue;
            }
            self.upsert(Document {
                id: Uuid::new_v4(),
                source_kind: s.kind,
                source_id: s.id.clone(),
                collected_at: s.collected_at,
                citation: s.citation.clone(),
                body: s.text.clone(),
                embedding: embedder.embed(&s.text),
            })?;
            n += 1;
        }
        Ok(n)
    }
}

impl DocumentStore for MemoryStore {
    fn upsert(&mut self, doc: Document) -> Result<()> {
        if let Some(existing) = self
            .docs
            .iter_mut()
            .find(|d| d.source_kind == doc.source_kind && d.source_id == doc.source_id)
        {
            *existing = doc;
        } else {
            self.docs.push(doc);
        }
        Ok(())
    }

    fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchHit>> {
        let mut hits: Vec<SearchHit> = self
            .docs
            .iter()
            .map(|d| SearchHit {
                source_id: d.source_id.clone(),
                source_kind: d.source_kind,
                citation: d.citation.clone(),
                body: d.body.clone(),
                score: cosine(query, &d.embedding),
            })
            .collect();
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(k.max(1));
        Ok(hits)
    }

    fn len(&self) -> usize {
        self.docs.len()
    }
}

pub fn record_to_document(s: &SourceRecord, embedder: &dyn Embedder) -> Document {
    Document {
        id: Uuid::new_v4(),
        source_kind: s.kind,
        source_id: s.id.clone(),
        collected_at: s.collected_at,
        citation: s.citation.clone(),
        body: s.text.clone(),
        embedding: embedder.embed(&s.text),
    }
}

pub fn default_embedder() -> HashingEmbedder {
    HashingEmbedder
}

#[cfg(feature = "postgres")]
pub mod postgres {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::PgPool;

    pub async fn connect(url: &str) -> Result<PgPool> {
        PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .map_err(|e| Error::Store(e.to_string()))
    }

    pub async fn migrate(pool: &PgPool) -> Result<()> {
        sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
            .execute(pool)
            .await
            .map_err(|e| Error::Store(e.to_string()))?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS documents (
                id uuid PRIMARY KEY,
                source_kind text NOT NULL,
                source_id text NOT NULL,
                collected_at timestamptz NOT NULL,
                citation text NOT NULL,
                body text NOT NULL,
                embedding vector(64) NOT NULL,
                metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
                UNIQUE (source_kind, source_id)
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| Error::Store(e.to_string()))?;
        Ok(())
    }

    pub struct PostgresStore {
        pool: PgPool,
    }

    impl PostgresStore {
        pub fn new(pool: PgPool) -> Self {
            Self { pool }
        }

        pub async fn upsert_async(&self, doc: &Document) -> Result<()> {
            let vec = pgvector::Vector::from(doc.embedding.clone());
            sqlx::query(
                r#"
                INSERT INTO documents (id, source_kind, source_id, collected_at, citation, body, embedding)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (source_kind, source_id) DO UPDATE SET
                    collected_at = EXCLUDED.collected_at,
                    citation = EXCLUDED.citation,
                    body = EXCLUDED.body,
                    embedding = EXCLUDED.embedding
                "#,
            )
            .bind(doc.id)
            .bind(doc.source_kind.as_str())
            .bind(&doc.source_id)
            .bind(doc.collected_at)
            .bind(&doc.citation)
            .bind(&doc.body)
            .bind(vec)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Store(e.to_string()))?;
            Ok(())
        }

        pub async fn search_async(&self, query: &[f32], k: usize) -> Result<Vec<SearchHit>> {
            let vec = pgvector::Vector::from(query.to_vec());
            let rows: Vec<(String, String, String, String)> = sqlx::query_as(
                r#"
                SELECT source_id, source_kind, citation, body
                FROM documents
                ORDER BY embedding <=> $1
                LIMIT $2
                "#,
            )
            .bind(vec)
            .bind(k as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Store(e.to_string()))?;
            Ok(rows
                .into_iter()
                .map(|(source_id, kind, citation, body)| SearchHit {
                    source_id,
                    source_kind: parse_kind(&kind),
                    citation,
                    body,
                    score: 0.0,
                })
                .collect())
        }
    }

    fn parse_kind(s: &str) -> SourceKind {
        parse_stored_kind(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn competitor_watch_kind_not_authoritative() {
        assert!(!SourceKind::CompetitorWatch.onchain_authoritative());
        assert_eq!(
            parse_stored_kind("competitor_watch"),
            SourceKind::CompetitorWatch
        );
    }

    #[test]
    fn search_finds_ingested_dex_volume() {
        let mut store = MemoryStore::new();
        let e = HashingEmbedder;
        let rec = SourceRecord {
            id: "dex-overview".into(),
            kind: SourceKind::DexIndexer,
            collected_at: Utc::now(),
            citation: "GET https://indexer.dex.cl8y.com/api/v1/overview".into(),
            text: "CL8Y DEX 24h volume_usd 12000 as of 2026-08-31T00:00:00Z".into(),
            numbers: vec![],
            degraded: false,
            gap: None,
            metadata: serde_json::json!({}),
        };
        store.ingest_sources(&[rec], &e).unwrap();
        let hits = store.search(&e.embed("dex volume"), 3).unwrap();
        assert_eq!(hits[0].source_id, "dex-overview");
        assert!(hits[0].score > 0.1);
    }
}
