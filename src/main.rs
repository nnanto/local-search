use anyhow::{Result};
use local_search_engine::{DocumentIndexer, JsonFileIngestor, LocalSearch, SqliteLocalSearchEngine};

fn main() -> Result<()> {
    env_logger::init();
    
    // First, create and use the indexer
    let search_engine = SqliteLocalSearchEngine::new("_local.db")?;
    search_engine.create_table()?;
    let file_indexer = JsonFileIngestor::new(Box::new(search_engine));
    file_indexer.ingest("/tmp/documents")?;
    
    // Then create a new search engine instance for searching
    let mut search_engine = SqliteLocalSearchEngine::new("_local.db")?;
    search_engine.refresh()?;
    println!("Total documents indexed: {}", search_engine.stats()?);
    
    let res = search_engine.search("service", local_search_engine::SearchType::FullText)?;
    for r in res {
        println!("Found document: {:?}", r);
    }
    
    Ok(())
}