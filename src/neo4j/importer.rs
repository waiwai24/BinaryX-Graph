use anyhow::Result;
use neo4rs::query;
use serde::{Deserialize, Serialize};

use super::Neo4jConnection;
use crate::models::{Function, StringNode, Library, Binary};

#[derive(Debug, Clone)]
pub struct ImportStatistics {
    pub binaries: usize,
    pub functions: usize,
    pub strings: usize,
    pub libraries: usize,
    pub calls_relationships: usize,
}

#[derive(Clone)]
pub struct GraphImporter {
    connection: Neo4jConnection,
}

impl GraphImporter {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self {
            connection,
        }
    }

    pub async fn get_statistics_async(&self) -> Result<ImportStatistics> {
        let mut stats = ImportStatistics {
            binaries: 0,
            functions: 0,
            strings: 0,
            libraries: 0,
            calls_relationships: 0,
        };

        // Count binaries
        let binary_query = "MATCH (b:Binary) RETURN count(b) as count";
        let mut result = self.connection.graph().execute(query(binary_query)).await?;
        if let Some(row) = result.next().await? {
            stats.binaries = row.get::<i64>("count").unwrap_or(0) as usize;
        }

        // Count functions
        let function_query = "MATCH (f:Function) RETURN count(f) as count";
        let mut result = self.connection.graph().execute(query(function_query)).await?;
        if let Some(row) = result.next().await? {
            stats.functions = row.get::<i64>("count").unwrap_or(0) as usize;
        }

        // Count strings
        let string_query = "MATCH (s:String) RETURN count(s) as count";
        let mut result = self.connection.graph().execute(query(string_query)).await?;
        if let Some(row) = result.next().await? {
            stats.strings = row.get::<i64>("count").unwrap_or(0) as usize;
        }

        // Count libraries
        let library_query = "MATCH (l:Library) RETURN count(l) as count";
        let mut result = self.connection.graph().execute(query(library_query)).await?;
        if let Some(row) = result.next().await? {
            stats.libraries = row.get::<i64>("count").unwrap_or(0) as usize;
        }

        // Count CALLS relationships
        let calls_query = "MATCH ()-[r:CALLS]->() RETURN count(r) as count";
        let mut result = self.connection.graph().execute(query(calls_query)).await?;
        if let Some(row) = result.next().await? {
            stats.calls_relationships = row.get::<i64>("count").unwrap_or(0) as usize;
        }

        Ok(stats)
    }

    pub fn connection(&self) -> &Neo4jConnection {
        &self.connection
    }

    pub async fn import_binary(&self, binary: &Binary) -> Result<()> {
        let query_str = "
            MERGE (b:Binary {hash: $hash})
            SET b.filename = $filename,
                b.file_path = $file_path,
                b.file_size = $file_size,
                b.format = $format,
                b.arch = $arch
        ";

        let format_str = format!("{:?}", binary.format);

        self.connection.graph().run(query(query_str)
            .param("hash", binary.hash.as_str())
            .param("filename", binary.filename.as_str())
            .param("file_path", binary.file_path.as_str())
            .param("file_size", binary.file_size as i64)
            .param("format", format_str.as_str())
            .param("arch", binary.arch.as_str())
        ).await?;

        Ok(())
    }

    pub async fn import_function(&self, function: &Function) -> Result<()> {
        let query_str = "
            MERGE (f:Function {uid: $uid})
            SET f.name = $name,
                f.address = $address,
                f.type = $type,
                f.size = $size
        ";

        let type_str = format!("{:?}", function.r#type);

        self.connection.graph().run(query(query_str)
            .param("uid", function.uid.as_str())
            .param("name", function.name.as_str())
            .param("address", function.address.as_deref().unwrap_or(""))
            .param("type", type_str.as_str())
            .param("size", function.size.map(|s| s as i64).unwrap_or(-1))
        ).await?;

        Ok(())
    }

    pub async fn import_functions_batch(&self, functions: &[Function]) -> Result<()> {
        for function in functions {
            self.import_function(function).await?;
        }
        Ok(())
    }

    pub async fn create_contains_relationship(&self, binary_hash: &str, function_uid: &str) -> Result<()> {
        let query_str = "
            MATCH (b:Binary {hash: $binary_hash}), (f:Function {uid: $function_uid})
            MERGE (b)-[:CONTAINS]->(f)
        ";

        self.connection.graph().run(query(query_str)
            .param("binary_hash", binary_hash)
            .param("function_uid", function_uid)
        ).await?;

        Ok(())
    }

    pub async fn create_belongs_to_relationship(&self, function_uid: &str, library_name: &str) -> Result<()> {
        let query_str = "
            MATCH (f:Function {uid: $function_uid}), (l:Library {name: $library_name})
            MERGE (f)-[:BELONGS_TO]->(l)
        ";

        self.connection.graph().run(query(query_str)
            .param("function_uid", function_uid)
            .param("library_name", library_name)
        ).await?;

        Ok(())
    }

    pub async fn import_string_node(&self, string_node: &StringNode) -> Result<()> {
        let query_str = "
            MERGE (s:String {uid: $uid})
            SET s.value = $value,
                s.address = $address
        ";

        self.connection.graph().run(query(query_str)
            .param("uid", string_node.uid.as_str())
            .param("value", string_node.value.as_str())
            .param("address", string_node.address.as_deref().unwrap_or(""))
        ).await?;

        Ok(())
    }

    pub async fn import_library(&self, library: &Library) -> Result<()> {
        let query_str = "
            MERGE (l:Library {name: $name})
        ";

        self.connection.graph().run(query(query_str)
            .param("name", library.name.as_str())
        ).await?;

        Ok(())
    }

    pub async fn create_imports_relationship(&self, binary_hash: &str, library_name: &str) -> Result<()> {
        let query_str = "
            MATCH (b:Binary {hash: $binary_hash}), (l:Library {name: $library_name})
            MERGE (b)-[:IMPORTS]->(l)
        ";

        self.connection.graph().run(query(query_str)
            .param("binary_hash", binary_hash)
            .param("library_name", library_name)
        ).await?;

        Ok(())
    }

    pub async fn create_calls_relationship(&self, calls: &crate::models::Calls, from_uid: &str, to_uid: &str) -> Result<()> {
        let query_str = "
            MATCH (from:Function {uid: $from_uid}), (to:Function {uid: $to_uid})
            MERGE (from)-[r:CALLS]->(to)
            SET r.offset = $offset,
                r.call_type = $call_type
        ";

        let call_type_str = format!("{:?}", calls.call_type);

        self.connection.graph().run(query(query_str)
            .param("from_uid", from_uid)
            .param("to_uid", to_uid)
            .param("offset", calls.offset.as_str())
            .param("call_type", call_type_str.as_str())
        ).await?;

        Ok(())
    }

    pub async fn query_functions(&self, pattern: &str, binary: Option<&str>) -> Result<Vec<Function>> {
        let query_str = if let Some(_binary_name) = binary {
            "
            MATCH (b:Binary)-[:CONTAINS]->(f:Function)
            WHERE (f.name CONTAINS $pattern OR f.uid CONTAINS $pattern)
              AND (b.filename CONTAINS $binary_name OR b.hash = $binary_name)
            RETURN f
            LIMIT 100
        "
        } else {
            "
            MATCH (f:Function)
            WHERE f.name CONTAINS $pattern OR f.uid CONTAINS $pattern
            RETURN f
            LIMIT 100
        "
        };

        let mut query_builder = query(query_str).param("pattern", pattern);
        if let Some(binary_name) = binary {
            query_builder = query_builder.param("binary_name", binary_name);
        }

        let mut result = self.connection.graph().execute(query_builder).await?;

        let mut functions = Vec::new();
        while let Some(row) = result.next().await? {
            if let Ok(node) = row.get::<neo4rs::Node>("f") {
                let type_str = node.get::<String>("type").unwrap_or_else(|_| "Internal".to_string());
                let r#type = match type_str.as_str() {
                    "Import" => crate::models::FunctionType::Import,
                    "Export" => crate::models::FunctionType::Export,
                    "Thunk" => crate::models::FunctionType::Thunk,
                    _ => crate::models::FunctionType::Internal,
                };

                let function = Function {
                    uid: node.get::<String>("uid").unwrap_or_default(),
                    name: node.get::<String>("name").unwrap_or_default(),
                    address: node.get::<String>("address").ok(),
                    r#type,
                    size: node.get::<i64>("size").ok().map(|s| s as u64),
                };
                functions.push(function);
            }
        }

        Ok(functions)
    }

    pub async fn query_binary_info(&self, binary_name: &str) -> Result<Option<Binary>> {
        let query_str = "
            MATCH (b:Binary)
            WHERE b.hash = $binary_name OR b.filename CONTAINS $binary_name
            RETURN b
            LIMIT 1
        ";

        let mut result = self.connection.graph().execute(
            query(query_str).param("binary_name", binary_name)
        ).await?;

        if let Some(row) = result.next().await? {
            if let Ok(node) = row.get::<neo4rs::Node>("b") {
                let format_str = node.get::<String>("format").unwrap_or_else(|_| "PE".to_string());
                let format = match format_str.as_str() {
                    "Elf" => crate::models::BinaryFormat::Elf,
                    "MachO" => crate::models::BinaryFormat::MachO,
                    _ => crate::models::BinaryFormat::PE,
                };

                let binary = Binary {
                    hash: node.get::<String>("hash").unwrap_or_default(),
                    filename: node.get::<String>("filename").unwrap_or_default(),
                    file_path: node.get::<String>("file_path").unwrap_or_default(),
                    file_size: node.get::<i64>("file_size").unwrap_or(0) as u64,
                    format,
                    arch: node.get::<String>("arch").unwrap_or_default(),
                };
                return Ok(Some(binary));
            }
        }

        Ok(None)
    }

    pub async fn query_callgraph_with_depth(&self, function_name: &str, binary: Option<&str>, max_depth: usize) -> Result<CallGraph> {
        let callees_query = if let Some(_binary_name) = binary {
            format!(
                "MATCH (b:Binary)-[:CONTAINS]->(f:Function)-[:CALLS*1..{}]->(callee:Function)
                 WHERE (f.name = $function_name OR f.uid = $function_name)
                   AND (b.filename CONTAINS $binary_name OR b.hash = $binary_name)
                 RETURN DISTINCT callee",
                max_depth
            )
        } else {
            format!(
                "MATCH (f:Function)-[:CALLS*1..{}]->(callee:Function)
                 WHERE f.name = $function_name OR f.uid = $function_name
                 RETURN DISTINCT callee",
                max_depth
            )
        };

        let mut query_builder = query(&callees_query).param("function_name", function_name);
        if let Some(binary_name) = binary {
            query_builder = query_builder.param("binary_name", binary_name);
        }

        let mut result = self.connection.graph().execute(query_builder).await?;

        let mut callees = Vec::new();
        while let Some(row) = result.next().await? {
            if let Ok(node) = row.get::<neo4rs::Node>("callee") {
                callees.push(FunctionInfo {
                    uid: node.get::<String>("uid").unwrap_or_default(),
                    name: node.get::<String>("name").unwrap_or_default(),
                    address: node.get::<String>("address").ok(),
                });
            }
        }

        let callers_query = if let Some(_binary_name) = binary {
            format!(
                "MATCH (b:Binary)-[:CONTAINS]->(f:Function)<-[:CALLS*1..{}]-(caller:Function)
                 WHERE (f.name = $function_name OR f.uid = $function_name)
                   AND (b.filename CONTAINS $binary_name OR b.hash = $binary_name)
                 RETURN DISTINCT caller",
                max_depth
            )
        } else {
            format!(
                "MATCH (caller:Function)-[:CALLS*1..{}]->(f:Function)
                 WHERE f.name = $function_name OR f.uid = $function_name
                 RETURN DISTINCT caller",
                max_depth
            )
        };

        let mut query_builder = query(&callers_query).param("function_name", function_name);
        if let Some(binary_name) = binary {
            query_builder = query_builder.param("binary_name", binary_name);
        }

        let mut result = self.connection.graph().execute(query_builder).await?;

        let mut callers = Vec::new();
        while let Some(row) = result.next().await? {
            if let Ok(node) = row.get::<neo4rs::Node>("caller") {
                callers.push(FunctionInfo {
                    uid: node.get::<String>("uid").unwrap_or_default(),
                    name: node.get::<String>("name").unwrap_or_default(),
                    address: node.get::<String>("address").ok(),
                });
            }
        }

        Ok(CallGraph { callees, callers })
    }

    pub async fn query_xrefs(&self, address: &str, binary: Option<&str>) -> Result<Vec<Xref>> {
        let query_str = if let Some(_binary_name) = binary {
            "
            MATCH (b:Binary)-[:CONTAINS]->(from:Function)-[r:CALLS]->(to:Function)
            WHERE (from.address = $address OR to.address = $address)
              AND (b.filename CONTAINS $binary_name OR b.hash = $binary_name)
            RETURN from.name as from_function, to.name as to_function, r.offset as offset
        "
        } else {
            "
            MATCH (from:Function)-[r:CALLS]->(to:Function)
            WHERE from.address = $address OR to.address = $address
            RETURN from.name as from_function, to.name as to_function, r.offset as offset
        "
        };

        let mut query_builder = query(query_str).param("address", address);
        if let Some(binary_name) = binary {
            query_builder = query_builder.param("binary_name", binary_name);
        }

        let mut result = self.connection.graph().execute(query_builder).await?;

        let mut xrefs = Vec::new();
        while let Some(row) = result.next().await? {
            if let (Ok(from), Ok(to), Ok(offset)) = (
                row.get::<String>("from_function"),
                row.get::<String>("to_function"),
                row.get::<String>("offset"),
            ) {
                xrefs.push(Xref {
                    from_function: from,
                    to_function: to,
                    offset,
                });
            }
        }

        Ok(xrefs)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    pub callees: Vec<FunctionInfo>,
    pub callers: Vec<FunctionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub uid: String,
    pub name: String,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Xref {
    pub from_function: String,
    pub to_function: String,
    pub offset: String,
}
