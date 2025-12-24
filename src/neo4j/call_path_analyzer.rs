use anyhow::Result;
use neo4rs::Query;

use crate::models::{
    CallPath, CallPathNode, EnhancedCallGraph, CallSequence, 
    UpwardCallChain, 
    UpwardCallNode, CallerSequence, CallContextAnalysis
};
use crate::neo4j::importer::FunctionInfo;

/// Call path analyzer
pub struct CallPathAnalyzer {
    connection: super::Neo4jConnection,
}

impl CallPathAnalyzer {
    pub fn new(connection: super::Neo4jConnection) -> Self {
        Self { connection }
    }

    pub async fn query_call_paths(&self, function_name: &str, max_depth: usize) -> Result<Vec<CallPath>> {
        let mut paths = Vec::new();

        let query = Query::new(format!(
            "MATCH path = (start:Function)-[:CALLS*1..{}]->(end:Function)
             WHERE start.name = $function_name OR start.uid = $function_name
             RETURN path, length(path) as path_length,
                    [node in nodes(path) | node.name] as node_names,
                    [node in nodes(path) | node.address] as node_addresses,
                    [rel in relationships(path) | rel.offset] as call_offsets",
            max_depth
        ))
        .param("function_name", function_name.to_string());

        let mut result = self.connection.graph().execute(query).await?;
        let mut path_counter = 0;

        while let Some(row) = result.next().await? {
            path_counter += 1;
            
            let node_names: Vec<String> = row.get("node_names").unwrap_or_default();
            let node_addresses: Vec<String> = row.get("node_addresses").unwrap_or_default();
            let call_offsets: Vec<String> = row.get("call_offsets").unwrap_or_default();
            
            if !node_names.is_empty() {
                let mut call_path = CallPath::new(format!("path_{}", path_counter));
                
                for (i, name) in node_names.iter().enumerate() {
                    let address = node_addresses.get(i).cloned().unwrap_or_else(|| "N/A".to_string());
                    let call_site = if i > 0 {
                        call_offsets.get(i - 1).cloned()
                    } else {
                        None
                    };
                    
                    let node = CallPathNode::new(
                        format!("{}_{}", name, i),
                        name.clone(),
                        Some(address),
                        i,
                        call_site,
                        "Direct".to_string(),
                    );
                    
                    call_path.add_node(node);
                }
                
                paths.push(call_path);
            }
        }

        if paths.is_empty() {
            let mut call_path = CallPath::new("single_path".to_string());
            call_path.add_node(CallPathNode::new(
                "single_node".to_string(),
                function_name.to_string(),
                Some("0x1000".to_string()),
                0,
                None,
                "Entry".to_string(),
            ));
            paths.push(call_path);
        }

        Ok(paths)
    }

    pub async fn query_enhanced_call_graph(&self, function_name: &str, max_depth: usize) -> Result<EnhancedCallGraph> {
        let mut enhanced_graph = EnhancedCallGraph::new();

        let basic_query = Query::new(format!(
            "MATCH (f:Function)-[:CALLS*1..{}]->(callee:Function)
             WHERE f.name = $function_name OR f.uid = $function_name
             RETURN DISTINCT callee",
            max_depth
        ))
        .param("function_name", function_name.to_string());

        let mut result = self.connection.graph().execute(basic_query).await?;

        while let Some(row) = result.next().await? {
            if let Ok(node) = row.get::<neo4rs::Node>("callee") {
                enhanced_graph.callees.push(FunctionInfo {
                    uid: node.get::<String>("uid").unwrap_or_default(),
                    name: node.get::<String>("name").unwrap_or_default(),
                    address: node.get::<String>("address").ok(),
                });
            }
        }

        let call_paths = self.query_call_paths(function_name, max_depth).await?;
        for path in call_paths {
            enhanced_graph.add_call_path(path);
        }

        let frequency_query = Query::new(
            "MATCH (caller:Function)-[:CALLS]->(callee:Function)
             WHERE caller.name = $function_name OR caller.uid = $function_name
             RETURN callee.name as callee_name, count(*) as frequency"
                .to_string(),
        )
        .param("function_name", function_name.to_string());

        let mut result = self.connection.graph().execute(frequency_query).await?;

        while let Some(row) = result.next().await? {
            if let (Ok(callee_name), Ok(frequency)) = (
                row.get::<String>("callee_name"),
                row.get::<i64>("frequency"),
            ) {
                enhanced_graph.set_call_frequency(callee_name, frequency);
            }
        }

        Ok(enhanced_graph)
    }

    

    /// Query call sequences (with order information)
    pub async fn query_call_sequences(&self, function_name: &str) -> Result<Vec<CallSequence>> {
        let mut sequences = Vec::new();

        // Query call sequences within the function
        let query = Query::new(
            "MATCH (f:Function)-[r:CALLS]->(callee:Function)
             WHERE f.name = $function_name OR f.uid = $function_name
             RETURN f.name as caller, callee.name as callee, r.offset as call_site
             ORDER BY r.offset"
                .to_string(),
        )
        .param("function_name", function_name.to_string());

        let mut result = self.connection.graph().execute(query).await?;
        let mut order_counter = 0;

        while let Some(row) = result.next().await? {
            if let (Ok(caller), Ok(callee), Ok(call_site)) = (
                row.get::<String>("caller"),
                row.get::<String>("callee"),
                row.get::<String>("call_site"),
            ) {
                order_counter += 1;
                
                let sequence = CallSequence::new(
                    format!("seq_{}", order_counter),
                    caller,
                    callee,
                    order_counter,
                    call_site,
                );
                
                sequences.push(sequence);
            }
        }

        Ok(sequences)
    }

    

    pub async fn find_recursive_calls(&self, function_name: &str) -> Result<Vec<RecursiveCall>> {
        let mut recursive_calls = Vec::new();

        let direct_query = Query::new(
            "MATCH (f:Function)-[:CALLS]->(f)
             WHERE f.name = $function_name OR f.uid = $function_name
             RETURN f.name as function_name, f.address as address"
                .to_string(),
        )
        .param("function_name", function_name.to_string());

        let mut result = self.connection.graph().execute(direct_query).await?;

        while let Some(row) = result.next().await? {
            if let Ok(func_name) = row.get::<String>("function_name") {
                recursive_calls.push(RecursiveCall {
                    function_name: func_name,
                    call_type: RecursiveCallType::Direct,
                    depth: 1,
                });
            }
        }

        let indirect_query = Query::new(
            "MATCH path = (f:Function)-[:CALLS*2..10]->(f)
             WHERE f.name = $function_name OR f.uid = $function_name
             RETURN length(path) as depth, f.name as function_name, f.address as address,
                    [node in nodes(path) | node.name] as path_nodes"
                .to_string(),
        )
        .param("function_name", function_name.to_string());

        let mut result = self.connection.graph().execute(indirect_query).await?;

        while let Some(row) = result.next().await? {
            if let (Ok(func_name), Ok(depth)) = (
                row.get::<String>("function_name"),
                row.get::<i64>("depth"),
            ) {
                recursive_calls.push(RecursiveCall {
                    function_name: func_name,
                    call_type: RecursiveCallType::Indirect,
                    depth: depth as usize,
                });
            }
        }

        Ok(recursive_calls)
    }

    /// Query upward call chain (who called this function)
    pub async fn query_upward_call_chain(&self, function_name: &str, max_depth: usize) -> Result<Vec<UpwardCallChain>> {
        let mut chains = Vec::new();

        // Query all call paths pointing to the target function
        let query = Query::new(format!(
            "MATCH path = (start:Function)-[:CALLS*1..{}]->(end:Function)
             WHERE end.name = $function_name OR end.uid = $function_name
             RETURN path, length(path) as path_length,
                    [node in nodes(path) | node.name] as node_names,
                    [node in nodes(path) | node.address] as node_addresses,
                    [rel in relationships(path) | rel.offset] as call_offsets
             ORDER BY path_length",
            max_depth
        ))
        .param("function_name", function_name.to_string());

        let mut result = self.connection.graph().execute(query).await?;
        let mut chain_counter = 0;

        while let Some(row) = result.next().await? {
            chain_counter += 1;
            
            // Get node names, addresses, and call offsets
            let node_names: Vec<String> = row.get("node_names").unwrap_or_default();
            let node_addresses: Vec<String> = row.get("node_addresses").unwrap_or_default();
            let call_offsets: Vec<String> = row.get("call_offsets").unwrap_or_default();
            
            if !node_names.is_empty() {
                let mut chain = UpwardCallChain::new(format!("upward_chain_{}", chain_counter));
                
                // Add nodes in reverse order (from caller to callee)
                for (i, name) in node_names.iter().enumerate() {
                    let address = node_addresses.get(i).cloned().unwrap_or_else(|| "N/A".to_string());
                    let call_site = if i < node_names.len() - 1 {
                        call_offsets.get(i).cloned()
                    } else {
                        None
                    };
                    
                    let node = UpwardCallNode::new(
                        format!("{}_{}", name, i),
                        name.clone(),
                        Some(address),
                        i,
                        call_site,
                        "Upward".to_string(),
                    );
                    
                    chain.add_node(node);
                }
                
                chains.push(chain);
            }
        }

        // If no upward call chains are found, create a basic single-node chain
        if chains.is_empty() {
            let mut chain = UpwardCallChain::new("single_upward_chain".to_string());
            chain.add_node(UpwardCallNode::new(
                "single_node".to_string(),
                function_name.to_string(),
                Some("0x1000".to_string()),
                0,
                None,
                "Root".to_string(),
            ));
            chains.push(chain);
        }

        Ok(chains)
    }

    /// Query caller sequences (who called who, in call order)
    pub async fn query_caller_sequences(&self, function_name: &str) -> Result<Vec<CallerSequence>> {
        let mut sequences = Vec::new();

        // Query all functions that call the target function
        let query = Query::new(
            "MATCH (caller:Function)-[r:CALLS]->(callee:Function)
             WHERE callee.name = $function_name OR callee.uid = $function_name
             RETURN caller.name as caller_name, caller.address as caller_address, 
                    r.offset as call_site, callee.name as callee_name, callee.address as callee_address
             ORDER BY r.offset"
                .to_string(),
        )
        .param("function_name", function_name.to_string());

        let mut result = self.connection.graph().execute(query).await?;
        let mut order_counter = 0;

        while let Some(row) = result.next().await? {
            if let (Ok(caller_name), Ok(caller_address), Ok(call_site), Ok(callee_name), Ok(callee_address)) = (
                row.get::<String>("caller_name"),
                row.get::<String>("caller_address"),
                row.get::<String>("call_site"),
                row.get::<String>("callee_name"),
                row.get::<String>("callee_address"),
            ) {
                order_counter += 1;
                
                let sequence = CallerSequence::new(
                    format!("caller_seq_{}", order_counter),
                    caller_name,
                    caller_address,
                    callee_name,
                    callee_address,
                    order_counter,
                    call_site,
                );
                
                sequences.push(sequence);
            }
        }

        Ok(sequences)
    }

    /// Analyze complete call context (upward and downward call relationships)
    pub async fn analyze_call_context(&self, function_name: &str, max_depth: usize) -> Result<CallContextAnalysis> {
        let upward_chains = self.query_upward_call_chain(function_name, max_depth).await?;
        let downward_paths = self.query_call_paths(function_name, max_depth).await?;
        let caller_sequences = self.query_caller_sequences(function_name).await?;

        let mut analysis = CallContextAnalysis::new(function_name.to_string());

        // Add upward call chains
        for chain in upward_chains {
            analysis.add_upward_chain(chain);
        }

        // Add downward call paths
        for path in downward_paths {
            analysis.add_downward_path(path);
        }

        // Add caller sequences
        for sequence in caller_sequences {
            analysis.add_caller_sequence(sequence);
        }

        // Generate call context analysis
        analysis.generate_context_insights();

        Ok(analysis)
    }
}



/// Recursive call information
#[derive(Debug, Clone)]
pub struct RecursiveCall {
    pub function_name: String,
    pub call_type: RecursiveCallType,
    pub depth: usize,
}

/// Recursive call type
#[derive(Debug, Clone)]
pub enum RecursiveCallType {
    /// Direct recursion
    Direct,
    /// Indirect recursion
    Indirect,
}