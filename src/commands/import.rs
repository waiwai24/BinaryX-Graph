use anyhow::Result;
use std::path::Path;

use crate::cli::ImportType;
use crate::config::Config;
use crate::api::{DataImporter, ImportResult, ImportStatistics};

pub async fn handle_import(import_type: ImportType, config: Config) -> Result<()> {
    let importer = DataImporter::new(&config).await?;

    match import_type {
        ImportType::Json { file_path, batch_size: _, no_validate } => {
            let result = import_single_file(&importer, &file_path, !no_validate).await?;
            print_import_result(&result);
        }
        ImportType::Directory { dir_path, pattern, batch_size, no_validate } => {
            import_directory(&importer, &dir_path, &pattern, batch_size, !no_validate).await?
        }
    }

    Ok(())
}

async fn import_single_file(
    importer: &DataImporter,
    file_path: &str,
    validate: bool,
) -> Result<ImportResult> {
    println!("Importing file: {}", file_path);

    if !Path::new(file_path).exists() {
        return Err(anyhow::anyhow!("File not found: {}", file_path));
    }

    if validate {
        println!("Validating data...");
        let data: serde_json::Value = {
            let file = std::fs::File::open(file_path)?;
            let reader = std::io::BufReader::new(file);
            serde_json::from_reader(reader)?
        };

        let validation = importer.validate_data(&data).await?;
        if !validation.valid {
            println!("Validation failed:");
            for error in &validation.errors {
                println!("  - {}", error);
            }
            return Err(anyhow::anyhow!("Data validation failed"));
        }

        if !validation.warnings.is_empty() {
            println!("Warnings:");
            for warning in &validation.warnings {
                println!("  - {}", warning);
            }
        }
        println!("Validation passed");
    }

    println!("Importing data...");
    let result = importer.import_from_file(file_path).await?;
    println!("Import completed");

    Ok(result)
}

fn print_import_result(result: &ImportResult) {
    println!("\nImport completed {}!", if result.success { "successfully" } else { "with errors" });
    println!("Statistics:");
    println!("  Binaries: {}", result.statistics.binaries);
    println!("  Functions: {}", result.statistics.functions);
    println!("  Strings: {}", result.statistics.strings);
    println!("  Libraries: {}", result.statistics.libraries);
    println!("  Call relationships: {}", result.statistics.calls_relationships);
    println!("  Total nodes: {}", result.statistics.total_nodes);

    if !result.errors.is_empty() {
        println!("\nErrors encountered:");
        for error in result.errors.iter().take(10) {
            println!("  - {}", error);
        }
        if result.errors.len() > 10 {
            println!("  ... and {} more errors", result.errors.len() - 10);
        }
    }
}

async fn import_directory(
    importer: &DataImporter,
    dir_path: &str,
    pattern: &str,
    batch_size: usize,
    validate: bool,
) -> Result<()> {
    println!("Importing directory: {}", dir_path);
    println!("Pattern: {}", pattern);
    println!("Batch size: {}", batch_size);

    if !Path::new(dir_path).exists() {
        return Err(anyhow::anyhow!("Directory not found: {}", dir_path));
    }

    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                if matches_pattern(file_name, pattern) {
                    files.push(path);
                }
            }
        }
    }

    if files.is_empty() {
        println!("No files found matching pattern: {}", pattern);
        return Ok(());
    }

    println!("Found {} files to import", files.len());

    let mut total_stats = ImportStatistics {
        binaries: 0,
        functions: 0,
        strings: 0,
        libraries: 0,
        calls_relationships: 0,
        total_nodes: 0,
    };
    let mut total_errors = Vec::new();
    let mut success_count = 0;
    let total_files = files.len();

    // Process files in batches
    let batches: Vec<_> = files.chunks(batch_size).collect();
    let total_batches = batches.len();

    for (batch_idx, batch) in batches.iter().enumerate() {
        println!("\n=== Processing batch {}/{} ({} files) ===",
            batch_idx + 1, total_batches, batch.len());

        let batch_start_idx = batch_idx * batch_size;

        for (file_idx, file_path) in batch.iter().enumerate() {
            let overall_idx = batch_start_idx + file_idx + 1;
            println!("[{}/{}] Importing {}...", overall_idx, total_files, file_path.display());

            match import_single_file(importer, &file_path.to_string_lossy(), validate).await {
                Ok(result) => {
                    total_stats.binaries += result.statistics.binaries;
                    total_stats.functions += result.statistics.functions;
                    total_stats.strings += result.statistics.strings;
                    total_stats.libraries += result.statistics.libraries;
                    total_stats.calls_relationships += result.statistics.calls_relationships;
                    total_stats.total_nodes += result.statistics.total_nodes;

                    for error in result.errors {
                        total_errors.push(format!("{}: {}", file_path.display(), error));
                    }

                    if result.success {
                        success_count += 1;
                    }
                }
                Err(e) => {
                    println!("Failed to import {}: {}", file_path.display(), e);
                    total_errors.push(format!("{}: {}", file_path.display(), e));
                }
            }
        }

        let batch_end_idx = batch_start_idx + batch.len();
        println!("Batch {}/{} completed. Progress: {}/{} files",
            batch_idx + 1, total_batches, batch_end_idx, total_files);
    }

    println!("\nDirectory import completed!");
    println!("Summary:");
    println!("  Files processed: {}/{}", success_count, total_files);
    println!("\nTotal Statistics:");
    println!("  Binaries: {}", total_stats.binaries);
    println!("  Functions: {}", total_stats.functions);
    println!("  Strings: {}", total_stats.strings);
    println!("  Libraries: {}", total_stats.libraries);
    println!("  Call relationships: {}", total_stats.calls_relationships);
    println!("  Total nodes: {}", total_stats.total_nodes);

    if !total_errors.is_empty() {
        println!("\nErrors encountered ({}):", total_errors.len());
        for error in total_errors.iter().take(10) {
            println!("  - {}", error);
        }
        if total_errors.len() > 10 {
            println!("  ... and {} more errors", total_errors.len() - 10);
        }
    }

    Ok(())
}

fn matches_pattern(filename: &str, pattern: &str) -> bool {
    if pattern == "*" || pattern == "*.*" {
        return true;
    }

    if let Some(ext_pattern) = pattern.strip_prefix("*.") {
        if let Some(ext) = filename.rsplit('.').next() {
            return ext.eq_ignore_ascii_case(ext_pattern);
        }
        return false;
    }

    if let Some(prefix) = pattern.strip_suffix('*') {
        return filename.starts_with(prefix);
    }

    if let Some(suffix) = pattern.strip_prefix('*') {
        return filename.ends_with(suffix);
    }

    filename == pattern
}
