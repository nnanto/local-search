use rusqlite::Connection;
use log::{info, debug};
use crate::{LocalEmbedder, traits::SearchResult};
use anyhow::anyhow;
use crate::traits::{DocumentIndexer, LocalSearch, DocumentRequest, SearchType};


pub struct SqliteLocalSearchEngine {
    db_path: String,
    conn: Connection,
    embedder: LocalEmbedder,
}


impl SqliteLocalSearchEngine {
    pub fn new(db_path: &str) -> anyhow::Result<Self> {
        info!("Creating new SqliteLocalSearch for path: {}", db_path);
        let conn = Connection::open(db_path).map_err(|e| anyhow!("Failed to open database: {}", e))?;
        let embedder = LocalEmbedder::new()?;
        let lfts = SqliteLocalSearchEngine { db_path: db_path.to_string(), conn, embedder };
        info!("SqliteLocalSearch initialization complete: {}", db_path);
        Ok(lfts)
    }

    pub fn create_table(&self) -> anyhow::Result<()> {
        self.conn.execute( "CREATE TABLE IF NOT EXISTS documents (
                    path TEXT PRIMARY KEY,
                    content TEXT NOT NULL,
                    metadata TEXT NOT NULL,
                    createdAt REAL NOT NULL,
                    updatedAt REAL NOT NULL
                )", [])?;
        debug!("Created documents table if it did not exist.");

        self.conn.execute("DROP TABLE IF EXISTS documents_fts", [])?;
        
        debug!("Dropped existing documents_fts table if it existed.");
        self.conn.execute(
            "CREATE VIRTUAL TABLE documents_fts USING fts5(
                path UNINDEXED,
                content
            )",
            [],
        )?;
        debug!("Created documents_fts FTS5 virtual table.");
        
        // Create embeddings table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS document_embeddings (
                path TEXT PRIMARY KEY,
                embedding BLOB NOT NULL,
                FOREIGN KEY(path) REFERENCES documents(path)
            )", 
            []
        )?;
        debug!("Created document_embeddings table if it did not exist.");
        
        // let schema: String = self.conn.query_one("SELECT sql FROM sqlite_main WHERE type='table' AND name='documents'", [], |row| row.get(0))?;
        // debug!("Documents table schema: {}", schema);
        // // Check if FTS table was created
        // let fts_exists: i32 = self.conn.query_row(
        //     "SELECT COUNT(*) FROM sqlite_main WHERE type='table' AND name='documents_fts'",
        //     [],
        //     |row| row.get(0),
        // )?;
        // info!("FTS table exists: {}", fts_exists > 0);
        Ok(())
    }

    pub fn stats(&self) -> anyhow::Result<i64> {
        let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))?;
        info!("Total documents indexed: {}", count);
        Ok(count)
    }

    fn search_semantic_only(&self, query: &str) -> anyhow::Result<Vec<SearchResult>> {
        let query_embedding = self.embedder.embed_text(query)?;
        let semantic_results = self.search_by_embedding(&query_embedding)?;
        let results = semantic_results.into_iter().map(|r| SearchResult {
            path: r.path,
            metadata: r.metadata,
            created_at: r.created_at,
            updated_at: r.updated_at,
            fts_score: None,
            semantic_score: Some(r.semantic_score.unwrap_or(0.0)),
            final_score: r.final_score,
        }).collect();
        Ok(results)
    }

    fn search_hybrid(&self, query: &str) -> anyhow::Result<Vec<SearchResult>> {
        // Get FTS results
        let fts_results = self.search_fts(query).unwrap_or_default();
        
        // Get semantic results
        let query_embedding = self.embedder.embed_text(query)?;
        let semantic_results = self.search_by_embedding(&query_embedding).unwrap_or_default();
        
        // Combine and normalize scores
        let mut combined_results = std::collections::HashMap::new();
        
        // Normalize FTS scores (convert to 0-1 range)
        let max_fts_score = fts_results.iter().map(|r| r.fts_score.unwrap_or(0.0)).fold(f64::NEG_INFINITY, f64::max);
        let min_fts_score = fts_results.iter().map(|r| r.fts_score.unwrap_or(0.0)).fold(f64::INFINITY, f64::min);
        let fts_range = if max_fts_score != min_fts_score { max_fts_score - min_fts_score } else { 1.0 };
        
        for result in fts_results {
            let normalized_score = if fts_range > 0.0 { 
                (result.fts_score.unwrap_or(0.0) - min_fts_score) / fts_range 
            } else { 
                1.0 
            };
            combined_results.insert(result.path.clone(), (
                result,
                Some(normalized_score),
                None,
            ));
        }
        
        // Semantic scores are already normalized (cosine similarity 0-1)
        for result in semantic_results {
            let result_score = result.semantic_score.unwrap_or(0.0); // Extract score before move
            match combined_results.get_mut(&result.path) {
                Some((_, _fts_score, semantic_score)) => {
                    *semantic_score = Some(result_score);
                }
                None => {
                    combined_results.insert(result.path.clone(), (
                        result,
                        None,
                        Some(result_score),
                    ));
                }
            }
        }
        
        // Calculate hybrid scores
        let mut final_results: Vec<SearchResult> = combined_results.into_iter().map(|(_, (base_result, fts_score, semantic_score))| {
            let fts_component = fts_score.unwrap_or(0.0) * 0.6;
            let semantic_component = semantic_score.unwrap_or(0.0) * 0.4;
            let final_score = fts_component + semantic_component;
            
            SearchResult {
                path: base_result.path,
                metadata: base_result.metadata.clone(),
                created_at: base_result.created_at,
                updated_at: base_result.updated_at,
                fts_score,
                semantic_score,
                final_score,
            }
        }).collect();
        
        // Sort by final score descending
        final_results.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal));
        
        debug!("Hybrid search for query '{}' returned {} results.", query, final_results.len());
        Ok(final_results)
    }

    fn search_by_embedding(&self, query_embedding: &[f32]) -> anyhow::Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT d.id, d.path, d.metadata, d.createdAt, d.updatedAt, e.embedding
             FROM documents d 
             JOIN document_embeddings e ON d.id = e.id"
        ).map_err(|e| anyhow!("Failed to prepare semantic search query: {}", e))?;
        
        let embedding_iter = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let path: String = row.get(1)?;
            let metadata_str: String = row.get(2)?;
            let metadata: Option<std::collections::HashMap<String, String>> = serde_json::from_str(&metadata_str).ok();
            let created_at: f64 = row.get(3)?;
            let updated_at: f64 = row.get(4)?;
            let embedding_bytes: Vec<u8> = row.get(5)?;
            Ok((id, path, metadata, created_at, updated_at, embedding_bytes))
        }).map_err(|e| anyhow!("Failed to query embeddings: {}", e))?;

        let mut results = Vec::new();
        for result in embedding_iter {
            let (_id, path, metadata, created_at, updated_at, embedding_bytes) = result.map_err(|e| anyhow!("Failed to read embedding row: {}", e))?;
            
            // Convert bytes back to f32 vector
            let embedding: Vec<f32> = embedding_bytes
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            
            // Calculate cosine similarity
            let similarity = self.cosine_similarity(query_embedding, &embedding);
            
            results.push(SearchResult {
                path,
                metadata,
                created_at,
                updated_at,
                fts_score: None,
                semantic_score: Some(similarity),
                final_score: similarity,
            });
        }
        
        // Sort by similarity score descending
        results.sort_by(|a, b| b.semantic_score.unwrap_or(0.0).partial_cmp(&a.semantic_score.unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
        
        debug!("Semantic search returned {} results.", results.len());
        Ok(results)
    }

    fn search_fulltext_only(&self, query: &str) -> anyhow::Result<Vec<SearchResult>> {
        let fts_results = self.search_fts(query)?;
        info!("Full-text search for query '{}' returned {} results.", query, fts_results.len());
        let results = fts_results.into_iter().map(|r| SearchResult {
            path: r.path,
            metadata: r.metadata,
            created_at: r.created_at,
            updated_at: r.updated_at,
            fts_score: Some(r.fts_score.unwrap_or(0.0)),
            semantic_score: None,
            final_score: r.final_score,
        }).collect();
        Ok(results)
    }

    fn search_fts(&self, query: &str) -> anyhow::Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT d.path, d.metadata, d.createdAt, d.updatedAt, bm25(documents_fts) * -1 as score
             FROM documents_fts 
             JOIN documents d ON documents_fts.path = d.path
             WHERE documents_fts MATCH ?1
             ORDER BY rank",
        )?;
        let search_iter = stmt.query_map(rusqlite::params![query], |row| {
            let score: f64 = row.get(4)?;
            Ok(SearchResult {
                path: row.get(0)?,
                metadata: serde_json::from_str(&row.get::<_, String>(1)?).ok(),
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                fts_score: Some(score),
                semantic_score: None,
                final_score: score,
            })
        })?;

        let mut results = Vec::new();
        for result in search_iter {
            results.push(result?);
        }
        debug!("Search for query '{}' returned {} results.", query, results.len());
        Ok(results)
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        // For normalized embeddings, cosine similarity is just the dot product
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        dot_product as f64
    }
    
}

impl DocumentIndexer for SqliteLocalSearchEngine {

    fn insert_document(&self, request: DocumentRequest) -> anyhow::Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        let metadata_str = serde_json::to_string(&request.metadata)
            .map_err(|e| anyhow!("Failed to serialize metadata: {}", e))?;
        let created_at = now;
        let updated_at = now;
        
        let rows_affected = self.conn.execute("INSERT INTO documents (path, content, metadata, createdAt, updatedAt) values (?1, ?2, ?3, ?4, ?5)", rusqlite::params![request.path, request.content, metadata_str, created_at, updated_at])
            .map_err(|e| anyhow!("Failed to insert document: {}", e))?;
        debug!("Inserted document with path: {}. Number of rows affected: {}", request.path, rows_affected);
        
        // Generate and store embedding
        let embedding = self.embedder.embed_text(&request.content)?;
        let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        self.conn.execute(
            "INSERT INTO document_embeddings (path, embedding) VALUES (?1, ?2)",
            rusqlite::params![request.path, embedding_bytes],
        ).map_err(|e| anyhow!("Failed to insert embedding: {}", e))?;
        debug!("Inserted embedding for document with path: {}", request.path);
        
        // Insert into FTS table for search
        self.conn.execute(
            "INSERT INTO documents_fts (path, content) VALUES (?1, ?2)",
            rusqlite::params![request.path, request.content],
        ).map_err(|e| anyhow!("Failed to insert into FTS: {}", e))?;
        debug!("Inserted document into FTS table with path: {}", request.path);
        Ok(())
    }

    fn upsert_document(&self, request: DocumentRequest) -> anyhow::Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        let metadata_str = serde_json::to_string(&request.metadata)
            .map_err(|e| anyhow!("Failed to serialize metadata: {}", e))?;
        let updated_at = now;
        
        let rows_affected = self.conn.execute(
            "UPDATE documents SET content = ?1, metadata = ?2, updatedAt = ?3 WHERE path = ?4",
            rusqlite::params![request.content, metadata_str, updated_at, request.path],
        ).map_err(|e| anyhow!("Failed to update document: {}", e))?;
        
        if rows_affected == 0 {
            // Document does not exist, insert new
            debug!("Document with path: {} did not exist. Inserting new document.", request.path);
            self.insert_document(request)?;
        } else {
            debug!("Updated document with path: {}. Number of rows affected: {}", request.path, rows_affected);
            
            // Update embedding
            let embedding = self.embedder.embed_text(&request.content)?;
            let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
            self.conn.execute(
                "UPDATE document_embeddings SET embedding = ?1 WHERE path = ?2",
                rusqlite::params![embedding_bytes, request.path],
            ).map_err(|e| anyhow!("Failed to update embedding: {}", e))?;
            debug!("Updated embedding for document with path: {}", request.path);
            
            // Update FTS table
            self.conn.execute(
                "UPDATE documents_fts SET content = ?1 WHERE path = ?2",
                rusqlite::params![request.content, request.path],
            ).map_err(|e| anyhow!("Failed to update FTS: {}", e))?;
            debug!("Updated FTS entry for document with path: {}", request.path);
        }
        Ok(())
    }

    fn delete_document(&self, path: &str) -> anyhow::Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM documents WHERE path = ?1",
            rusqlite::params![path],
        ).map_err(|e| anyhow!("Failed to delete document: {}", e))?;
        debug!("Deleted document with path: {}. Number of rows affected: {}", path, rows_affected);
        
        self.conn.execute(
            "DELETE FROM document_embeddings WHERE path = ?1",
            rusqlite::params![path],
        ).map_err(|e| anyhow!("Failed to delete embedding: {}", e))?;
        debug!("Deleted embedding for document with path: {}", path);
        
        self.conn.execute(
            "DELETE FROM documents_fts WHERE path = ?1",
            rusqlite::params![path],
        ).map_err(|e| anyhow!("Failed to delete from FTS: {}", e))?;
        debug!("Deleted FTS entry for document with path: {}", path);
        Ok(())
    }

    fn refresh(&mut self) -> anyhow::Result<()> {
        // Close and reopen the connection to refresh from underlying database changes
        let db_path = self.db_path.clone();
        let new_conn = Connection::open(&db_path).map_err(|e| anyhow!("Failed to reopen database: {}", e))?;
        let old_conn = std::mem::replace(&mut self.conn, new_conn);
        old_conn.close().map_err(|e| anyhow!("Failed to close database connection: {}", e.1))?;
        info!("Database connection refreshed for path: {:?}", self.db_path);
        Ok(())
    }
}

impl LocalSearch for SqliteLocalSearchEngine {
    
    fn search(&self, query: &str, search_type: SearchType) -> anyhow::Result<Vec<SearchResult>> {
        match search_type {
            SearchType::FullText => self.search_fulltext_only(query),
            SearchType::Semantic => self.search_semantic_only(query),
            SearchType::Hybrid => self.search_hybrid(query),
        }
    }

}

