use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::neo4j::importer::FunctionInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallPathNode {
    pub id: String,
    pub name: String,
    pub address: Option<String>,
    pub depth: usize,
    pub call_site: Option<String>,
    pub call_type: String,
}

impl CallPathNode {
    pub fn new(
        id: String,
        name: String,
        address: Option<String>,
        depth: usize,
        call_site: Option<String>,
        call_type: String,
    ) -> Self {
        Self {
            id,
            name,
            address,
            depth,
            call_site,
            call_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallPath {
    pub id: String,
    pub nodes: Vec<CallPathNode>,
    pub length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSequence {
    pub id: String,
    pub caller: String,
    pub callee: String,
    pub order: usize,
    pub call_site: String,
}


impl CallPath {
    pub fn new(id: String) -> Self {
        Self {
            id,
            nodes: Vec::new(),
            length: 0,
        }
    }

    pub fn add_node(&mut self, node: CallPathNode) {
        self.length = node.depth;
        self.nodes.push(node);
    }

    pub fn entry_function(&self) -> Option<&CallPathNode> {
        self.nodes.first()
    }
}


impl CallSequence {
    pub fn new(
        id: String,
        caller: String,
        callee: String,
        order: usize,
        call_site: String,
    ) -> Self {
        Self {
            id,
            caller,
            callee,
            order,
            call_site,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpwardCallNode {
    pub id: String,
    pub name: String,
    pub address: Option<String>,
    pub depth: usize,
    pub call_site: Option<String>,
    pub call_type: String,
}

impl UpwardCallNode {
    pub fn new(
        id: String,
        name: String,
        address: Option<String>,
        depth: usize,
        call_site: Option<String>,
        call_type: String,
    ) -> Self {
        Self {
            id,
            name,
            address,
            depth,
            call_site,
            call_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpwardCallChain {
    pub id: String,
    pub nodes: Vec<UpwardCallNode>,
    pub length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallerSequence {
    pub id: String,
    pub caller_name: String,
    pub caller_address: String,
    pub callee_name: String,
    pub callee_address: String,
    pub order: usize,
    pub call_site: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallContextAnalysis {
    pub function_name: String,
    pub upward_chains: Vec<UpwardCallChain>,
    pub downward_paths: Vec<CallPath>,
    pub caller_sequences: Vec<CallerSequence>,
    pub context_insights: Vec<String>,
}

impl UpwardCallChain {
    pub fn new(id: String) -> Self {
        Self {
            id,
            nodes: Vec::new(),
            length: 0,
        }
    }

    pub fn add_node(&mut self, node: UpwardCallNode) {
        self.length = node.depth;
        self.nodes.push(node);
    }

    pub fn target_function(&self) -> Option<&UpwardCallNode> {
        self.nodes.first()
    }
}

impl CallerSequence {
    pub fn new(
        id: String,
        caller_name: String,
        caller_address: String,
        callee_name: String,
        callee_address: String,
        order: usize,
        call_site: String,
    ) -> Self {
        Self {
            id,
            caller_name,
            caller_address,
            callee_name,
            callee_address,
            order,
            call_site,
        }
    }
}

impl CallContextAnalysis {
    pub fn new(function_name: String) -> Self {
        Self {
            function_name,
            upward_chains: Vec::new(),
            downward_paths: Vec::new(),
            caller_sequences: Vec::new(),
            context_insights: Vec::new(),
        }
    }

    pub fn add_upward_chain(&mut self, chain: UpwardCallChain) {
        self.upward_chains.push(chain);
    }

    pub fn add_downward_path(&mut self, path: CallPath) {
        self.downward_paths.push(path);
    }

    pub fn add_caller_sequence(&mut self, sequence: CallerSequence) {
        self.caller_sequences.push(sequence);
    }

    pub fn generate_context_insights(&mut self) {
        self.context_insights.push(format!(
            "Function '{}' has {} upward call chains and {} downward call paths",
            self.function_name,
            self.upward_chains.len(),
            self.downward_paths.len()
        ));

        if !self.caller_sequences.is_empty() {
            self.context_insights.push(format!(
                "Function is called by {} different callers",
                self.caller_sequences.len()
            ));
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedCallGraph {
    pub callees: Vec<FunctionInfo>,
    pub call_paths: Vec<CallPath>,
    pub call_frequencies: HashMap<String, i64>,
}

impl EnhancedCallGraph {
    pub fn new() -> Self {
        Self {
            callees: Vec::new(),
            call_paths: Vec::new(),
            call_frequencies: HashMap::new(),
        }
    }

    pub fn add_call_path(&mut self, path: CallPath) {
        self.call_paths.push(path);
    }

    pub fn set_call_frequency(&mut self, callee_name: String, frequency: i64) {
        self.call_frequencies.insert(callee_name, frequency);
    }
}