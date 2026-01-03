use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchType {
    FullText,
    Semantic,
    Hybrid,
}

#[derive(Debug)]
pub struct SearchResult {
    pub path: String,
    pub metadata: Option<std::collections::HashMap<String, String>>,
    pub created_at: f64,
    pub updated_at: f64,
    pub fts_score: Option<f64>,
    pub semantic_score: Option<f64>,
    pub final_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRequest {
    pub path: String,
    pub content: String,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

pub trait DocumentIndexer {
    fn insert_document(&self, request: DocumentRequest) -> anyhow::Result<()>;
    fn upsert_document(&self, request: DocumentRequest) -> anyhow::Result<()>;
    fn delete_document(&self, path: &str) -> anyhow::Result<()>;
    fn refresh(&mut self) -> anyhow::Result<()>;
}

pub trait LocalSearch {
    fn search(&self, query: &str, search_type: SearchType) -> anyhow::Result<Vec<SearchResult>>;
}