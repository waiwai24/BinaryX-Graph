use anyhow::Result;
use std::io::{self, Write};

use crate::cli::DatabaseAction;
use crate::config::Config;
use crate::api::DataImporter;
use crate::neo4j::SchemaManager;

pub async fn handle_database(db_action: DatabaseAction, config: Config) -> Result<()> {
    match db_action {
        DatabaseAction::Init => {
            init_database(&config).await?
        }
        DatabaseAction::Clear { confirm } => {
            clear_database(&config, confirm).await?
        }
        DatabaseAction::Stats => {
            show_database_stats(&config).await?
        }
        DatabaseAction::Export { output_path, format } => {
            export_database(&config, &output_path, &format).await?
        }
    }

    Ok(())
}

async fn init_database(config: &Config) -> Result<()> {
    println!("Initializing database schema...");

    let connection = crate::neo4j::Neo4jConnection::new(config).await?;
    
    // Test connectivity first
    println!("Testing Neo4j connectivity...");
    connection.verify_connectivity().await?;
    println!("Neo4j connection successful");

    // Initialize schema
    SchemaManager::initialize_database(&connection).await?;
    
    println!("Database schema initialized successfully");
    Ok(())
}

async fn clear_database(config: &Config, confirm: bool) -> Result<()> {
    if !confirm {
        print!("This will delete ALL data in the database. Are you sure? [y/N]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().to_lowercase().starts_with('y') {
            println!("Operation cancelled");
            return Ok(());
        }
    }

    println!("Clearing database...");

    let connection = crate::neo4j::Neo4jConnection::new(config).await?;
    SchemaManager::clear_database(&connection).await?;
    
    println!("Database cleared successfully");
    Ok(())
}

async fn show_database_stats(config: &Config) -> Result<()> {
    println!("Retrieving database statistics...");

    // Use DataImporter to get statistics (read from MemoryStore)
    let importer = DataImporter::new(config).await?;
    let stats = importer.get_database_stats().await?;

    println!("\nDatabase Statistics:");
    println!("  Total nodes: {}", stats.node_count);
    println!("  Total relationships: {}", stats.relationship_count);

    println!("\nNodes by type:");
    for (label, count) in &stats.label_counts {
        println!("  {}: {}", label, count);
    }

    // Get additional import statistics
    let import_stats = importer.get_import_statistics().await?;

    println!("\nImport Statistics:");
    println!("  Binaries: {}", import_stats.binaries);
    println!("  Functions: {}", import_stats.functions);
    println!("  Strings: {}", import_stats.strings);
    println!("  Libraries: {}", import_stats.libraries);
    println!("  Call relationships: {}", import_stats.calls_relationships);

    Ok(())
}

async fn export_database(config: &Config, output_path: &str, format: &str) -> Result<()> {
    println!("Exporting database to {} (format: {})", output_path, format);

    let importer = DataImporter::new(config).await?;
    
    match format {
        "json" => {
            importer.export_to_json(output_path).await?;
            println!("Database exported to JSON: {}", output_path);
        }
        "csv" => {
            return Err(anyhow::anyhow!("CSV export not yet implemented"));
        }
        _ => {
            return Err(anyhow::anyhow!("Unsupported export format: {}", format));
        }
    }

    Ok(())
}