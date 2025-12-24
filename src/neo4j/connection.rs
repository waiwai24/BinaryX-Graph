use anyhow::{Context, Result};
use neo4rs::{ConfigBuilder, Graph, Query};
use std::sync::Arc;

use super::DatabaseStats;

#[derive(Clone)]
pub struct Neo4jConnection {
    graph: Arc<Graph>,
}

impl Neo4jConnection {
    pub async fn new(config: &crate::config::Config) -> Result<Self> {
        let mut config_builder = ConfigBuilder::default()
            .uri(&config.neo4j_uri)
            .user(&config.neo4j_user)
            .password(&config.neo4j_password);

        // Set custom database if specified
        if let Some(ref db_name) = config.neo4j_database {
            config_builder = config_builder.db(db_name.as_str());
        }

        let neo4j_config = config_builder
            .build()
            .context("Failed to build Neo4j configuration")?;

        let graph = Graph::connect(neo4j_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to Neo4j database: {}", e))?;

        Ok(Self {
            graph: Arc::new(graph),
        })
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub async fn test_connection(&self) -> Result<()> {
        let query = Query::new("RETURN 1 as test".to_string());
        let _ = self
            .graph
            .execute(query)
            .await
            .context("Failed to execute test query")?;
        Ok(())
    }

    pub async fn verify_connectivity(&self) -> Result<()> {
        self.test_connection().await
    }

    pub async fn get_database_stats(&self) -> Result<DatabaseStats> {
        let mut stats = DatabaseStats::new();

        let node_count_query = Query::new("MATCH (n) RETURN count(n) as count".to_string());
        let mut result = self.graph.execute(node_count_query).await?;
        if let Some(row) = result.next().await? {
            stats.node_count = row.get::<i64>("count").unwrap_or(0);
        }

        let rel_count_query = Query::new("MATCH ()-[r]->() RETURN count(r) as count".to_string());
        let mut result = self.graph.execute(rel_count_query).await?;
        if let Some(row) = result.next().await? {
            stats.relationship_count = row.get::<i64>("count").unwrap_or(0);
        }

        let labels = ["Binary", "Function", "String", "Library"];
        for label in labels {
            let query = Query::new(format!("MATCH (n:{}) RETURN count(n) as count", label));
            let mut result = self.graph.execute(query).await?;
            if let Some(row) = result.next().await? {
                let count = row.get::<i64>("count").unwrap_or(0);
                stats.label_counts.insert(label.to_string(), count);
            }
        }

        Ok(stats)
    }

    pub async fn execute_query(
        &self,
        cypher: &str,
        params: Option<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>> {
        let mut query = Query::new(cypher.to_string());

        if let Some(serde_json::Value::Object(map)) = params {
            for (key, value) in map {
                query = match value {
                    serde_json::Value::String(s) => query.param(&key, s),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            query.param(&key, i)
                        } else if let Some(f) = n.as_f64() {
                            query.param(&key, f)
                        } else {
                            query
                        }
                    }
                    serde_json::Value::Bool(b) => query.param(&key, b),
                    _ => query,
                };
            }
        }

        let mut result = self.graph.execute(query).await?;
        let mut rows = Vec::new();

        while let Some(row) = result.next().await? {
            let mut json_row = serde_json::Map::new();

            if let Ok(node) = row.get::<neo4rs::Node>("n") {
                let mut node_map = serde_json::Map::new();
                let labels: Vec<String> = node.labels().iter().map(|s| s.to_string()).collect();
                node_map.insert("labels".to_string(), serde_json::json!(labels));
                if let Ok(uid) = node.get::<String>("uid") {
                    node_map.insert("uid".to_string(), serde_json::json!(uid));
                }
                if let Ok(name) = node.get::<String>("name") {
                    node_map.insert("name".to_string(), serde_json::json!(name));
                }
                if let Ok(hash) = node.get::<String>("hash") {
                    node_map.insert("hash".to_string(), serde_json::json!(hash));
                }
                if let Ok(address) = node.get::<String>("address") {
                    node_map.insert("address".to_string(), serde_json::json!(address));
                }
                if let Ok(value) = node.get::<String>("value") {
                    node_map.insert("value".to_string(), serde_json::json!(value));
                }
                json_row.insert("node".to_string(), serde_json::Value::Object(node_map));
            }

            if let Ok(rel_type) = row.get::<String>("rel_type") {
                json_row.insert("relationship_type".to_string(), serde_json::json!(rel_type));
            }

            if let Ok(target) = row.get::<neo4rs::Node>("m") {
                let mut target_map = serde_json::Map::new();
                let labels: Vec<String> = target.labels().iter().map(|s| s.to_string()).collect();
                target_map.insert("labels".to_string(), serde_json::json!(labels));
                if let Ok(uid) = target.get::<String>("uid") {
                    target_map.insert("uid".to_string(), serde_json::json!(uid));
                }
                if let Ok(name) = target.get::<String>("name") {
                    target_map.insert("name".to_string(), serde_json::json!(name));
                }
                json_row.insert("target".to_string(), serde_json::Value::Object(target_map));
            }

            if !json_row.is_empty() {
                rows.push(serde_json::Value::Object(json_row));
            }
        }

        Ok(rows)
    }

    pub async fn execute_write(&self, cypher: &str) -> Result<()> {
        let query = Query::new(cypher.to_string());
        let mut result = self.graph.execute(query).await?;
        while (result.next().await?).is_some() {}
        Ok(())
    }

    pub async fn clear_all(&self) -> Result<()> {
        let query = Query::new("MATCH (n) DETACH DELETE n".to_string());
        let mut result = self.graph.execute(query).await?;
        while (result.next().await?).is_some() {}
        Ok(())
    }
}
