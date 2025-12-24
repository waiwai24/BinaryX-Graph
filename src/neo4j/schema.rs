use super::Neo4jConnection;
use anyhow::Result;

pub struct SchemaManager {
    connection: Neo4jConnection,
}

impl SchemaManager {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    pub async fn initialize_schema(&self) -> Result<()> {
        self.create_constraints().await?;
        self.create_indexes().await?;
        Ok(())
    }

    pub async fn create_constraints(&self) -> Result<()> {
        let constraints = [
            // Binary node hash unique constraint
            "CREATE CONSTRAINT binary_hash_unique IF NOT EXISTS FOR (b:Binary) REQUIRE b.hash IS UNIQUE",
            // Function node uid unique constraint
            "CREATE CONSTRAINT function_uid_unique IF NOT EXISTS FOR (f:Function) REQUIRE f.uid IS UNIQUE",
            // String node uid unique constraint
            "CREATE CONSTRAINT string_uid_unique IF NOT EXISTS FOR (s:String) REQUIRE s.uid IS UNIQUE",
            // Library node name unique constraint
            "CREATE CONSTRAINT library_name_unique IF NOT EXISTS FOR (l:Library) REQUIRE l.name IS UNIQUE",
        ];

        for constraint in constraints {
            if let Err(e) = self.connection.execute_write(constraint).await {
                // Ignore constraint already exists errors
                eprintln!("[WARN] Constraint creation: {}", e);
            }
        }

        Ok(())
    }

    pub async fn create_indexes(&self) -> Result<()> {
        let indexes = [
            // Function indexes
            "CREATE INDEX function_name_index IF NOT EXISTS FOR (f:Function) ON (f.name)",
            "CREATE INDEX function_address_index IF NOT EXISTS FOR (f:Function) ON (f.address)",
            // Binary indexes
            "CREATE INDEX binary_filename_index IF NOT EXISTS FOR (b:Binary) ON (b.filename)",
            // String indexes
            "CREATE INDEX string_value_index IF NOT EXISTS FOR (s:String) ON (s.value)",
            // Fulltext indexes (for substring/keyword search)
            "CREATE FULLTEXT INDEX string_value_fulltext IF NOT EXISTS FOR (s:String) ON EACH [s.value]",
        ];

        for index in indexes {
            if let Err(e) = self.connection.execute_write(index).await {
                // Ignore index already exists errors
                eprintln!("[WARN] Index creation: {}", e);
            }
        }

        Ok(())
    }
    pub async fn initialize_database(connection: &Neo4jConnection) -> Result<()> {
        // Test connection
        connection.test_connection().await?;

        let manager = SchemaManager::new(connection.clone());
        manager.initialize_schema().await?;

        Ok(())
    }

    pub async fn clear_database(connection: &Neo4jConnection) -> Result<()> {
        connection.clear_all().await
    }
}
