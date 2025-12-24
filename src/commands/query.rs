use anyhow::Result;

use crate::api::DataImporter;
use crate::cli::QueryType;
use crate::config::Config;
use crate::neo4j::call_path_analyzer::RecursiveCallType;

#[derive(Debug)]
struct CallPathQueryConfig<'a> {
    binary: Option<&'a str>,
    show_paths: bool,
    show_sequences: bool,
    show_recursive: bool,
    show_upward: bool,
    show_context: bool,
    max_depth: usize,
    format: &'a str,
}

pub async fn handle_query(query_type: QueryType, config: Config) -> Result<()> {
    let importer = DataImporter::new(&config).await?;
    let session = importer.session();

    match query_type {
        QueryType::Functions {
            pattern,
            binary,
            limit,
            format,
        } => query_functions(&session, &pattern, binary.as_deref(), limit, &format).await?,
        QueryType::Binary {
            binary_name,
            format,
        } => query_binary(&session, &binary_name, &format).await?,
        QueryType::Callgraph {
            function_name,
            binary,
            show_callees,
            show_callers,
            max_depth,
            format,
        } => {
            query_callgraph(
                &session,
                &function_name,
                binary.as_deref(),
                show_callees,
                show_callers,
                max_depth,
                &format,
            )
            .await?
        }
        QueryType::Xrefs {
            address,
            binary,
            format,
        } => query_xrefs(&session, &address, binary.as_deref(), &format).await?,
        QueryType::CallPath {
            function_name,
            binary,
            show_paths,
            show_sequences,
            show_recursive,
            show_upward,
            show_context,
            max_depth,
            format,
        } => {
            query_call_paths(
                &session,
                &function_name,
                CallPathQueryConfig {
                    binary: binary.as_deref(),
                    show_paths,
                    show_sequences,
                    show_recursive,
                    show_upward,
                    show_context,
                    max_depth,
                    format: &format,
                },
            )
            .await?
        }
    }

    Ok(())
}

async fn query_functions(
    session: &crate::api::ImportSession,
    pattern: &str,
    binary: Option<&str>,
    limit: usize,
    format: &str,
) -> Result<()> {
    if let Some(binary_name) = binary {
        println!(
            "Querying functions with pattern: '{}' in binary: '{}'",
            pattern, binary_name
        );
    } else {
        println!("Querying functions with pattern: '{}'", pattern);
    }

    let functions = session.query_functions(pattern, binary).await?;
    let functions: Vec<_> = functions.into_iter().take(limit).collect();

    if functions.is_empty() {
        println!("No functions found matching pattern: '{}'", pattern);
        return Ok(());
    }

    if format == "json" {
        let json = serde_json::to_string_pretty(&functions)?;
        println!("{}", json);
    } else {
        println!("\nFunctions ({} found):", functions.len());
        println!(
            "{:<40} {:<20} {:<15} {:<20} {:<15}",
            "Name", "Type", "Address", "Binary", "UID"
        );
        println!("{}", "-".repeat(110));

        for f in &functions {
            let binary_display = extract_binary_from_uid(&f.uid);
            println!(
                "{:<40} {:<20} {:<15} {:<20} {:<15}",
                f.name,
                format!("{:?}", f.r#type),
                f.address.as_deref().unwrap_or("N/A"),
                binary_display,
                &f.uid[..f.uid.len().min(15)]
            );
        }
    }

    Ok(())
}

fn extract_binary_from_uid(uid: &str) -> &str {
    if let Some(colon_pos) = uid.find(':') {
        &uid[..colon_pos.min(15)]
    } else {
        &uid[..uid.len().min(15)]
    }
}

async fn query_binary(
    session: &crate::api::ImportSession,
    binary_name: &str,
    format: &str,
) -> Result<()> {
    println!("Querying binary with name pattern: '{}'", binary_name);

    if let Some(binary) = session.query_binary_info(binary_name).await? {
        if format == "json" {
            let json = serde_json::to_string_pretty(&binary)?;
            println!("{}", json);
        } else {
            println!("\nBinary Information:");
            println!("  Hash: {}", binary.hash);
            println!("  Filename: {}", binary.filename);
            println!("  Format: {:?}", binary.format);
            println!("  Architecture: {}", binary.arch);
        }
    } else {
        println!("No binary found matching pattern: '{}'", binary_name);
    }

    Ok(())
}

async fn query_callgraph(
    session: &crate::api::ImportSession,
    function_name: &str,
    binary: Option<&str>,
    show_callees: bool,
    show_callers: bool,
    max_depth: usize,
    format: &str,
) -> Result<()> {
    if let Some(binary_name) = binary {
        println!(
            "Querying call graph for function: '{}' in binary: '{}' (depth: {})",
            function_name, binary_name, max_depth
        );
    } else {
        println!(
            "Querying call graph for function: '{}' (depth: {})",
            function_name, max_depth
        );
    }

    let callgraph = session
        .query_callgraph_with_depth(function_name, binary, max_depth)
        .await?;

    let (display_callees, display_callers) = if !show_callees && !show_callers {
        (true, true)
    } else {
        (show_callees, show_callers)
    };

    if format == "json" {
        let json = serde_json::to_string_pretty(&callgraph)?;
        println!("{}", json);
    } else {
        if display_callees && !callgraph.callees.is_empty() {
            println!("\nCallees (functions called by '{}'):", function_name);
            println!("{:<40} {:<15}", "Name", "Address");
            println!("{}", "-".repeat(55));

            for f in &callgraph.callees {
                println!(
                    "{:<40} {:<15}",
                    f.name,
                    f.address.as_deref().unwrap_or("N/A")
                );
            }
        }

        if display_callers && !callgraph.callers.is_empty() {
            println!("\nCallers (functions calling '{}'):", function_name);
            println!("{:<40} {:<15}", "Name", "Address");
            println!("{}", "-".repeat(55));

            for f in &callgraph.callers {
                println!(
                    "{:<40} {:<15}",
                    f.name,
                    f.address.as_deref().unwrap_or("N/A")
                );
            }
        }

        if callgraph.callees.is_empty() && callgraph.callers.is_empty() {
            println!(
                "No call graph information found for function: '{}'",
                function_name
            );
        } else {
            println!(
                "\nSummary: {} callees, {} callers",
                callgraph.callees.len(),
                callgraph.callers.len()
            );
        }
    }

    Ok(())
}

async fn query_xrefs(
    session: &crate::api::ImportSession,
    address: &str,
    binary: Option<&str>,
    format: &str,
) -> Result<()> {
    if let Some(binary_name) = binary {
        println!(
            "Querying cross-references for address: '{}' in binary: '{}'",
            address, binary_name
        );
    } else {
        println!("Querying cross-references for address: '{}'", address);
    }

    let xrefs = session.query_xrefs(address, binary).await?;

    if xrefs.is_empty() {
        println!("No cross-references found for address: '{}'", address);
        return Ok(());
    }

    if format == "json" {
        let json = serde_json::to_string_pretty(&xrefs)?;
        println!("{}", json);
    } else {
        println!("\nCross-references ({} found):", xrefs.len());
        println!(
            "{:<30} {:<30} {:<15}",
            "From Function", "To Function", "Offset"
        );
        println!("{}", "-".repeat(75));

        for x in &xrefs {
            println!(
                "{:<30} {:<30} {:<15}",
                x.from_function, x.to_function, x.offset
            );
        }
    }

    Ok(())
}

async fn query_call_paths(
    session: &crate::api::ImportSession,
    function_name: &str,
    config: CallPathQueryConfig<'_>,
) -> Result<()> {
    if let Some(binary_name) = config.binary {
        println!(
            "Analyzing call paths and execution order for function: '{}' in binary: '{}'",
            function_name, binary_name
        );
    } else {
        println!(
            "Analyzing call paths and execution order for function: '{}'",
            function_name
        );
    }

    let analyzer = crate::neo4j::CallPathAnalyzer::new(session.importer().connection().clone());

    let show_all = !config.show_paths
        && !config.show_sequences
        && !config.show_recursive
        && !config.show_upward
        && !config.show_context;

    if config.show_paths || show_all {
        println!("\nAnalyzing call paths...");
        let call_paths = analyzer
            .query_call_paths(function_name, config.max_depth)
            .await?;

        if call_paths.is_empty() {
            println!("No call paths found");
        } else {
            let mut depth_stats = std::collections::HashMap::new();
            for path in &call_paths {
                *depth_stats.entry(path.length).or_insert(0) += 1;
            }

            println!("Found {} call paths:", call_paths.len());
            let mut sorted_depths: Vec<_> = depth_stats.iter().collect();
            sorted_depths.sort_by(|a, b| a.0.cmp(b.0));
            for (depth, count) in sorted_depths {
                println!("  Depth {}: {} paths", depth, count);
            }

            let mut sorted_paths = call_paths.clone();
            sorted_paths.sort_by(|a, b| b.length.cmp(&a.length));

            println!("\nLongest call path examples (top 10):");
            for (i, path) in sorted_paths.iter().take(10).enumerate() {
                if let Some(entry) = path.entry_function() {
                    println!("  Path {}: {} (Depth: {})", i + 1, entry.name, path.length);

                    if config.format == "json" {
                        let json = serde_json::to_string_pretty(&path)?;
                        println!("    Path details: {}", json);
                    } else {
                        for node in &path.nodes {
                            let indent = "  ".repeat(node.depth + 2);
                            println!(
                                "{}├─ {} @ {}",
                                indent,
                                node.name,
                                node.address.as_deref().unwrap_or("N/A")
                            );
                        }
                        println!();
                    }
                }
            }

            if call_paths.len() > 10 {
                println!(
                    "  ... and {} more paths (use --format json to see full list)",
                    call_paths.len() - 10
                );
            }
        }
    }

    if config.show_sequences || show_all {
        println!("\nAnalyzing call sequences...");
        let sequences = analyzer.query_call_sequences(function_name).await?;

        if sequences.is_empty() {
            println!("No call sequences found");
        } else {
            println!("Call execution order:");
            for sequence in &sequences {
                println!(
                    "  {}. {} -> {} (called at {})",
                    sequence.order, sequence.caller, sequence.callee, sequence.call_site
                );
            }
        }
    }

    if config.show_recursive || show_all {
        println!("\nChecking recursive calls...");
        let recursive_calls = analyzer.find_recursive_calls(function_name).await?;

        if recursive_calls.is_empty() {
            println!("No recursive calls found");
        } else {
            println!("Found {} recursive calls:", recursive_calls.len());
            for recursive in &recursive_calls {
                match recursive.call_type {
                    RecursiveCallType::Direct => {
                        println!(
                            "  Direct recursion: {} (Depth: {})",
                            recursive.function_name, recursive.depth
                        );
                    }
                    RecursiveCallType::Indirect => {
                        println!(
                            "  Indirect recursion: {} (Depth: {})",
                            recursive.function_name, recursive.depth
                        );
                    }
                }
            }
        }
    }

    if config.show_upward || show_all {
        println!("\nAnalyzing upward call chains...");
        let upward_chains = analyzer
            .query_upward_call_chain(function_name, config.max_depth)
            .await?;

        if upward_chains.is_empty() {
            println!("No upward call chains found");
        } else {
            let mut depth_stats = std::collections::HashMap::new();
            for chain in &upward_chains {
                *depth_stats.entry(chain.length).or_insert(0) += 1;
            }

            println!("Found {} upward call chains:", upward_chains.len());
            let mut sorted_depths: Vec<_> = depth_stats.iter().collect();
            sorted_depths.sort_by(|a, b| a.0.cmp(b.0));
            for (depth, count) in sorted_depths {
                println!("  Depth {}: {} call chains", depth, count);
            }

            let mut sorted_chains = upward_chains.clone();
            sorted_chains.sort_by(|a, b| b.length.cmp(&a.length));

            println!("\nDeepest upward call chain examples (top 10):");
            for (i, chain) in sorted_chains.iter().take(10).enumerate() {
                if let Some(target) = chain.target_function() {
                    println!(
                        "  Call chain {}: {} (Depth: {})",
                        i + 1,
                        target.name,
                        chain.length
                    );

                    if config.format == "json" {
                        let json = serde_json::to_string_pretty(&chain)?;
                        println!("    Call chain details: {}", json);
                    } else {
                        for node in &chain.nodes {
                            let indent = "  ".repeat((chain.length - node.depth) + 2);
                            let arrow = if node.depth < chain.length - 1 {
                                "├─"
                            } else {
                                "└─"
                            };
                            println!(
                                "{}{} {} @ {}",
                                indent,
                                arrow,
                                node.name,
                                node.address.as_deref().unwrap_or("N/A")
                            );
                        }
                        println!();
                    }
                }
            }

            if upward_chains.len() > 10 {
                println!(
                    "  ... and {} more call chains (use --format json to see full list)",
                    upward_chains.len() - 10
                );
            }

            let caller_sequences = analyzer.query_caller_sequences(function_name).await?;
            if !caller_sequences.is_empty() {
                println!("\nWho calls '{}':", function_name);
                for sequence in &caller_sequences {
                    println!(
                        "  {}. {} -> {} (called at {})",
                        sequence.order,
                        sequence.caller_name,
                        sequence.callee_name,
                        sequence.call_site
                    );
                }
            }
        }
    }

    if config.show_context || show_all {
        println!("\nFull call context analysis...");
        let context_analysis = analyzer
            .analyze_call_context(function_name, config.max_depth)
            .await?;

        println!("Call context insights:");
        for insight in &context_analysis.context_insights {
            println!("  {}", insight);
        }
    }

    if config.format == "json" {
        let enhanced_graph = analyzer
            .query_enhanced_call_graph(function_name, config.max_depth)
            .await?;
        let json = serde_json::to_string_pretty(&enhanced_graph)?;
        println!("\nEnhanced call graph (JSON):");
        println!("{}", json);
    }

    Ok(())
}
