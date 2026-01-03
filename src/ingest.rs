use crate::DocumentRequest;
use serde_json;
use std::path::Path;
use log::{debug, info, error};

pub struct JsonFileIngestor {
    pub indexer: Box<dyn crate::DocumentIndexer>,
}

///
/// expected json structure: DocumentRequest
/// 
/// 
impl JsonFileIngestor {

    pub fn new(indexer: Box<dyn crate::DocumentIndexer>) -> Self {
        JsonFileIngestor { indexer }
    }

    pub fn ingest(&self, path_str: &str) -> anyhow::Result<()> {
        let path = Path::new(path_str);

        std::fs::metadata(&path).expect("Path does not exist");
        info!("Starting ingestion with path: {}", path_str);
        
        if path.is_dir() {
            let mut processed_files = 0;
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let file_path = entry.path();
                debug!("Processing file: {:?}", file_path);
                if file_path.is_file() && 
                   file_path.extension().and_then(|s| s.to_str()) == Some("json") {
                    self.process_json_file(&file_path)?;
                    processed_files += 1;
                }
                else{
                    debug!("Skipping non-JSON file: {:?}", file_path);
                }
            }
            info!("Processed {} JSON files.", processed_files);
        } else {
            self.process_json_file(path)?;
            info!("Processed single JSON file: {:?}", path);
        }
        
        Ok(())
    }
    
    fn process_json_file(&self, file_path: &Path) -> anyhow::Result<()> {
        let data = std::fs::read_to_string(file_path)?;
        let doc_requests: Vec<DocumentRequest> = serde_json::from_str(&data)?;
        for doc_request in doc_requests {
            let content = doc_request.content.clone();
            self.indexer.upsert_document(doc_request)?;
            info!("Indexed document from file: {:?} -- content: {}", file_path, content);
        }
        Ok(())
    }
}

