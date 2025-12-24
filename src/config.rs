use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
    pub neo4j_database: Option<String>,
    pub batch_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            neo4j_uri: "bolt://localhost:7687".to_string(),
            neo4j_user: "neo4j".to_string(),
            neo4j_password: "password".to_string(),
            neo4j_database: None,
            batch_size: 1000,
        }
    }
}

impl Config {
    pub fn load_from_path(config_path: Option<&str>) -> Result<Self> {
        let path = config_path.unwrap_or("config.json");
        Self::load_from_file(path)
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let content = std::fs::read_to_string(path_ref).map_err(|e| {
            anyhow::anyhow!("Failed to read config file '{}': {}", path_ref.display(), e)
        })?;
        let config: Config = serde_json::from_str(&content).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse config file '{}': {}",
                path_ref.display(),
                e
            )
        })?;

        config.validate()?;

        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.neo4j_uri.is_empty() {
            return Err(anyhow::anyhow!("Neo4j URI cannot be empty"));
        }

        if self.neo4j_user.is_empty() {
            return Err(anyhow::anyhow!("Neo4j user cannot be empty"));
        }

        if self.neo4j_password.is_empty() {
            return Err(anyhow::anyhow!("Neo4j password cannot be empty"));
        }

        if self.batch_size == 0 {
            return Err(anyhow::anyhow!("Batch size must be greater than 0"));
        }

        Ok(())
    }
}
