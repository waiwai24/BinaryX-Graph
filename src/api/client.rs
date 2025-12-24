use anyhow::Result;
use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::config::Config;
use crate::neo4j::{Neo4jConnection, GraphImporter};

use super::ImportSession;

#[derive(Clone)]
pub struct DataImporter {
    connection: Neo4jConnection,
    importer: GraphImporter,
}

impl DataImporter {
    pub async fn new(config: &Config) -> Result<Self> {
        let connection = Neo4jConnection::new(config).await?;
        let importer = GraphImporter::new(connection.clone());

        Ok(Self {
            connection,
            importer,
        })
    }

    pub async fn import_from_file<P: AsRef<Path>>(&self, file_path: P) -> Result<ImportResult> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let data: Value = serde_json::from_reader(reader)?;

        self.import_from_json(data).await
    }

    pub async fn import_from_json(&self, data: Value) -> Result<ImportResult> {
        let session = ImportSession::new(self.importer.clone());
        session.import_data(data).await
    }

    pub async fn validate_data(&self, data: &Value) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let warnings = Vec::new();

        if let Some(binary_info) = data.get("binary_info") {
            if !binary_info.is_object() {
                errors.push("binary_info must be an object".to_string());
            } else {
                let required_fields = ["name", "file_path", "file_size", "file_type", "hashes"];
                for field in &required_fields {
                    if binary_info.get(field).is_none() {
                        errors.push(format!("binary_info missing required field: {}", field));
                    }
                }
            }
        } else {
            errors.push("binary_info is required".to_string());
        }

        if let Some(functions) = data.get("functions") {
            if !functions.is_array() {
                errors.push("functions must be an array".to_string());
            }
        }

        let array_fields = ["strings", "imports", "exports"];
        for field in &array_fields {
            if let Some(value) = data.get(field) {
                if !value.is_array() {
                    errors.push(format!("{} must be an array", field));
                }
            }
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    pub async fn get_import_statistics(&self) -> Result<ImportStatistics> {
        let stats = self.importer.get_statistics_async().await?;

        Ok(ImportStatistics {
            binaries: stats.binaries as i64,
            functions: stats.functions as i64,
            strings: stats.strings as i64,
            libraries: stats.libraries as i64,
            calls_relationships: stats.calls_relationships as i64,
            total_nodes: (stats.binaries + stats.functions + stats.strings + stats.libraries) as i64,
        })
    }

    pub async fn export_to_json<P: AsRef<Path>>(&self, file_path: P) -> Result<()> {
        let query = "MATCH (n) OPTIONAL MATCH (n)-[r]->(m) RETURN n, type(r) as rel_type, m";
        let results = self.connection.execute_query(query, None).await?;

        let json_string = serde_json::to_string_pretty(&results)?;
        std::fs::write(file_path, json_string)?;

        Ok(())
    }

    pub fn session(&self) -> ImportSession {
        ImportSession::new(self.importer.clone())
    }

    pub async fn get_database_stats(&self) -> Result<crate::neo4j::DatabaseStats> {
        self.connection.get_database_stats().await
    }
}

#[derive(Debug, Clone)]
pub struct ImportResult {
    pub success: bool,
    pub statistics: ImportStatistics,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImportStatistics {
    pub binaries: i64,
    pub functions: i64,
    pub strings: i64,
    pub libraries: i64,
    pub calls_relationships: i64,
    pub total_nodes: i64,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
