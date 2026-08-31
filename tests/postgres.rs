#![cfg(feature = "postgres")]

use chrono::Utc;
use cl8y_research::embed::HashingEmbedder;
use cl8y_research::store::postgres::{connect, migrate, PostgresStore};
use cl8y_research::store::{Document, SourceKind};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires DATABASE_URL and pgvector"]
async fn postgres_roundtrip() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = connect(&url).await.unwrap();
    migrate(&pool).await.unwrap();
    let store = PostgresStore::new(pool);
    let e = HashingEmbedder;
    let body = "CL8Y DEX 24h volume_usd 12000 as of 2026-08-31T00:00:00Z";
    store
        .upsert_async(&Document {
            id: Uuid::new_v4(),
            source_kind: SourceKind::DexIndexer,
            source_id: "dex-overview-test".into(),
            collected_at: Utc::now(),
            citation: "GET /api/v1/overview".into(),
            body: body.into(),
            embedding: e.embed(body),
        })
        .await
        .unwrap();
    let hits = store.search_async(&e.embed("dex volume"), 5).await.unwrap();
    assert!(hits.iter().any(|h| h.source_id == "dex-overview-test"));
}
