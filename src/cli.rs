use clap::{Parser, Subcommand};

use crate::commands;
use crate::config::Config;

#[derive(Parser)]
pub struct Cli {
    #[arg(short, long, global = true)]
    pub config: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Import JSON data into Neo4j
    Import {
        #[command(subcommand)]
        import_type: ImportType,
    },
    /// Query data from Neo4j
    Query {
        #[command(subcommand)]
        query_type: QueryType,
    },
    /// Database operations
    Database {
        #[command(subcommand)]
        db_action: DatabaseAction,
    },
}

#[derive(Subcommand)]
pub enum ImportType {
    /// Import JSON file
    Json {
        file_path: String,
        #[arg(long, default_value = "1000")]
        batch_size: usize,
        #[arg(long)]
        no_validate: bool,
    },
    /// Import directory of JSON files
    Directory {
        dir_path: String,
        #[arg(long, default_value = "*.json")]
        pattern: String,
        #[arg(long, default_value = "1000")]
        batch_size: usize,
        #[arg(long)]
        no_validate: bool,
    },
}

#[derive(Subcommand)]
pub enum QueryType {
    /// Query functions
    Functions {
        #[arg(long, default_value = "")]
        pattern: String,
        #[arg(long)]
        binary: Option<String>,
        #[arg(long, default_value = "100")]
        limit: usize,
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Query strings (fulltext search)
    Strings {
        /// Search text (auto converted to a Lucene wildcard query unless --raw is set)
        #[arg(long, default_value = "")]
        pattern: String,
        #[arg(long)]
        binary: Option<String>,
        #[arg(long, default_value = "100")]
        limit: usize,
        /// Treat pattern as a raw Lucene query
        #[arg(long)]
        raw: bool,
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Query binary information
    Binary {
        #[arg(long)]
        binary_name: String,
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Query call graph
    Callgraph {
        function_name: String,
        #[arg(long)]
        binary: Option<String>,
        #[arg(long)]
        show_callees: bool,
        #[arg(long)]
        show_callers: bool,
        #[arg(long, default_value = "1")]
        max_depth: usize,
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Query cross-references
    Xrefs {
        address: String,
        #[arg(long)]
        binary: Option<String>,
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Query call paths and execution order
    CallPath {
        function_name: String,
        #[arg(long)]
        binary: Option<String>,
        #[arg(long)]
        show_paths: bool,
        #[arg(long)]
        show_sequences: bool,
        #[arg(long)]
        show_recursive: bool,
        #[arg(long)]
        show_upward: bool,
        #[arg(long)]
        show_context: bool,
        #[arg(long, default_value = "5")]
        max_depth: usize,
        #[arg(long, default_value = "table")]
        format: String,
    },
}

#[derive(Subcommand)]
pub enum DatabaseAction {
    /// Initialize database schema
    Init,
    /// Clear all data
    Clear {
        #[arg(long)]
        confirm: bool,
    },
    /// Show database statistics
    Stats,
    /// Export data
    Export {
        output_path: String,
        #[arg(long, default_value = "json")]
        format: String,
    },
}

impl Cli {
    pub async fn execute(self, config: Config) -> anyhow::Result<()> {
        match self.command {
            Commands::Import { import_type } => {
                commands::import::handle_import(import_type, config).await
            }
            Commands::Query { query_type } => {
                commands::query::handle_query(query_type, config).await
            }
            Commands::Database { db_action } => {
                commands::database::handle_database(db_action, config).await
            }
        }
    }
}
