//! Agent-facing vector search over ingested sources.

use crate::embed::Embedder;
use crate::error::Result;
use crate::store::{DocumentStore, SearchHit};

pub fn search(
    store: &dyn DocumentStore,
    embedder: &dyn Embedder,
    query: &str,
    k: usize,
) -> Result<Vec<SearchHit>> {
    store.search(&embedder.embed(query), k)
}
