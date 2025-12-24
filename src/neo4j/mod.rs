pub mod connection;
pub mod importer;
pub mod schema;
pub mod call_path_analyzer;

pub use connection::Neo4jConnection;
pub use importer::{GraphImporter, CallGraph, Xref};
pub use schema::SchemaManager;
pub use call_path_analyzer::CallPathAnalyzer;


use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub node_count: i64,
    pub relationship_count: i64,
    pub label_counts: HashMap<String, i64>,
}

impl DatabaseStats {
    pub fn new() -> Self {
        Self {
            node_count: 0,
            relationship_count: 0,
            label_counts: HashMap::new(),
        }
    }
}

impl Default for DatabaseStats {
    fn default() -> Self {
        Self::new()
    }
}