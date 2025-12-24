use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

use crate::models::*;
use crate::neo4j::{CallGraph, GraphImporter, Xref};
use crate::utils::uid::{normalize_address, parse_address};

pub struct ImportSession {
    importer: GraphImporter,
}

impl ImportSession {
    pub fn new(importer: GraphImporter) -> Self {
        Self { importer }
    }

    pub async fn import_data(&self, data: Value) -> Result<crate::api::ImportResult> {
        let mut errors = Vec::new();
        let mut stats = crate::api::ImportStatistics {
            binaries: 0,
            functions: 0,
            strings: 0,
            libraries: 0,
            calls_relationships: 0,
            total_nodes: 0,
        };

        let mut address_to_uid: HashMap<String, String> = HashMap::new();

        let binary_info = match data.get("binary_info") {
            Some(info) => info,
            None => {
                errors.push("Missing binary_info in data".to_string());
                return Ok(crate::api::ImportResult {
                    success: false,
                    statistics: stats,
                    errors,
                });
            }
        };

        let binary = match self.parse_binary_info(binary_info) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!("Failed to parse binary info: {}", e));
                return Ok(crate::api::ImportResult {
                    success: false,
                    statistics: stats,
                    errors,
                });
            }
        };

        self.importer.import_binary(&binary).await?;
        stats.binaries = 1;
        let binary_hash = binary.hash.clone();

        if let Some(functions_data) = data.get("functions") {
            match self.parse_functions(functions_data, &binary_hash) {
                Ok(functions) => {
                    stats.functions += functions.len() as i64;

                    for function in &functions {
                        if let Some(address) = &function.address {
                            if let Some(normalized) = normalize_address(address) {
                                address_to_uid.insert(normalized, function.uid.clone());
                            }
                            address_to_uid.insert(address.clone(), function.uid.clone());
                        }
                    }

                    for chunk in functions.chunks(1000) {
                        self.importer.import_functions_batch(chunk).await?;

                        for function in chunk {
                            if let Err(e) = self
                                .importer
                                .create_contains_relationship(&binary_hash, &function.uid)
                                .await
                            {
                                errors
                                    .push(format!("Failed to create CONTAINS relationship: {}", e));
                            }
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("Failed to parse functions: {}", e));
                }
            }
        }

        if let Some(strings_data) = data.get("strings") {
            match self.parse_strings(strings_data, &binary_hash) {
                Ok(string_nodes) => {
                    let mut unique_strings: HashMap<String, StringNode> = HashMap::new();
                    for string_node in string_nodes {
                        unique_strings
                            .entry(string_node.uid.clone())
                            .or_insert(string_node);
                    }

                    let unique_count = unique_strings.len();
                    stats.strings += unique_count as i64;

                    for string_node in unique_strings.values() {
                        if let Err(e) = self.importer.import_string_node(string_node).await {
                            errors.push(format!("Failed to import string: {}", e));
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("Failed to parse strings: {}", e));
                }
            }
        }

        if let Some(imports_data) = data.get("imports") {
            match self.parse_imports(imports_data) {
                Ok((libraries, imports)) => {
                    stats.libraries += libraries.len() as i64;

                    for library in &libraries {
                        if let Err(e) = self.importer.import_library(library).await {
                            errors.push(format!("Failed to import library: {}", e));
                        }
                        // Create Binary-IMPORTS->Library relationship
                        if let Err(e) = self
                            .importer
                            .create_imports_relationship(&binary_hash, &library.name)
                            .await
                        {
                            errors.push(format!("Failed to create IMPORTS relationship: {}", e));
                        }
                    }

                    for import in &imports {
                        let lib_name_lower = import.library.to_lowercase();
                        let function = Function::create_import_with_address(
                            &binary_hash,
                            &lib_name_lower,
                            &import.name,
                            &import.address,
                        );

                        if let Some(normalized) = normalize_address(&import.address) {
                            address_to_uid.insert(normalized, function.uid.clone());
                        }
                        address_to_uid.insert(import.address.clone(), function.uid.clone());

                        if let Err(e) = self.importer.import_function(&function).await {
                            errors.push(format!("Failed to import function: {}", e));
                        }
                        if let Err(e) = self
                            .importer
                            .create_belongs_to_relationship(&function.uid, &lib_name_lower)
                            .await
                        {
                            errors.push(format!("Failed to create BELONGS_TO relationship: {}", e));
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("Failed to parse imports: {}", e));
                }
            }
        }

        if let Some(exports_data) = data.get("exports") {
            match self.parse_exports(exports_data) {
                Ok(exports) => {
                    for export in exports {
                        let address = match parse_address(&export.address) {
                            Some(addr) => addr,
                            None => {
                                errors.push(format!("Invalid export address: {}", export.address));
                                continue;
                            }
                        };
                        let function =
                            Function::create_internal(&binary_hash, address, &export.name, true);

                        if let Some(func_addr) = &function.address {
                            if !address_to_uid.contains_key(func_addr) {
                                if let Some(normalized) = normalize_address(func_addr) {
                                    address_to_uid.insert(normalized, function.uid.clone());
                                }
                                address_to_uid.insert(func_addr.clone(), function.uid.clone());
                            }
                        }

                        if let Err(e) = self.importer.import_function(&function).await {
                            errors.push(format!("Failed to import export function: {}", e));
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("Failed to parse exports: {}", e));
                }
            }
        }

        if let Some(calls_data) = data.get("calls") {
            match self
                .import_calls_with_mapping(calls_data, &address_to_uid)
                .await
            {
                Ok(call_count) => {
                    stats.calls_relationships += call_count;
                }
                Err(e) => {
                    errors.push(format!("Failed to import calls: {}", e));
                }
            }
        }

        stats.total_nodes = stats.binaries + stats.functions + stats.strings + stats.libraries;

        Ok(crate::api::ImportResult {
            success: errors.is_empty(),
            statistics: stats,
            errors,
        })
    }

    fn parse_binary_info(&self, binary_info: &Value) -> Result<Binary> {
        let hashes = binary_info
            .get("hashes")
            .ok_or_else(|| anyhow::anyhow!("Missing hashes"))?;

        let sha256 = hashes
            .get("sha256")
            .and_then(|v| v.as_str())
            .or_else(|| hashes.get("SHA256").and_then(|v| v.as_str()))
            .ok_or_else(|| anyhow::anyhow!("Missing sha256 hash"))?;

        let filename = binary_info
            .get("name")
            .and_then(|v| v.as_str())
            .or_else(|| binary_info.get("filename").and_then(|v| v.as_str()))
            .ok_or_else(|| anyhow::anyhow!("Missing filename"))?;

        let file_path = binary_info
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let file_size = binary_info
            .get("file_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let file_type = binary_info
            .get("file_type")
            .ok_or_else(|| anyhow::anyhow!("Missing file_type"))?;

        let format_str = file_type
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing file type"))?;

        let format_upper = format_str.to_uppercase();
        let format = if format_upper.contains("PE") {
            BinaryFormat::PE
        } else if format_upper.contains("ELF") {
            BinaryFormat::Elf
        } else if format_upper.contains("MACH") {
            BinaryFormat::MachO
        } else {
            BinaryFormat::PE // Default fallback
        };

        let arch = file_type
            .get("architecture")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        Ok(Binary {
            hash: sha256.to_string(),
            filename: filename.to_string(),
            file_path: file_path.to_string(),
            file_size,
            format,
            arch: arch.to_string(),
        })
    }

    fn parse_functions(&self, functions_data: &Value, binary_hash: &str) -> Result<Vec<Function>> {
        let functions_array = functions_data
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("functions must be an array"))?;

        let mut functions = Vec::with_capacity(functions_array.len());

        for func_data in functions_array {
            let name = func_data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let address_str = func_data
                .get("address")
                .and_then(|v| v.as_str())
                .unwrap_or("0x0");

            let address = parse_address(address_str).unwrap_or(0);

            let size = func_data.get("size").and_then(|v| v.as_u64());

            let mut function = Function::create_internal(binary_hash, address, name, false);
            function.size = size;
            functions.push(function);
        }

        Ok(functions)
    }

    fn parse_strings(&self, strings_data: &Value, binary_hash: &str) -> Result<Vec<StringNode>> {
        let strings_array = strings_data
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("strings must be an array"))?;

        let mut string_nodes = Vec::with_capacity(strings_array.len());

        for string_data in strings_array {
            let value = if let Some(v) = string_data.get("value").and_then(|v| v.as_str()) {
                v
            } else if let Some(v) = string_data.as_str() {
                v
            } else {
                continue;
            };

            let address = string_data
                .get("address")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let string_node = StringNode::new(binary_hash, value.to_string(), address);
            string_nodes.push(string_node);
        }

        Ok(string_nodes)
    }

    fn parse_imports(&self, imports_data: &Value) -> Result<(Vec<Library>, Vec<Import>)> {
        let imports_array = imports_data
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("imports must be an array"))?;

        let mut libraries: HashMap<String, Library> = HashMap::new();
        let mut imports = Vec::with_capacity(imports_array.len());

        for import_data in imports_array {
            let name = import_data
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Import missing name"))?;

            let library = import_data
                .get("library")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Import missing library"))?;

            let address = import_data
                .get("address")
                .and_then(|v| v.as_str())
                .unwrap_or("0x0");

            let lib_lower = library.to_lowercase();
            libraries
                .entry(lib_lower.clone())
                .or_insert_with(|| Library::create(&lib_lower));

            imports.push(Import {
                name: name.to_string(),
                address: address.to_string(),
                library: library.to_string(),
            });
        }

        let libraries_vec: Vec<Library> = libraries.into_values().collect();
        Ok((libraries_vec, imports))
    }

    fn parse_exports(&self, exports_data: &Value) -> Result<Vec<Export>> {
        let exports_array = exports_data
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("exports must be an array"))?;

        let mut exports = Vec::with_capacity(exports_array.len());

        for export_data in exports_array {
            let name = export_data
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Export missing name"))?;

            let address = export_data
                .get("address")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Export missing address"))?;

            exports.push(Export {
                name: name.to_string(),
                address: address.to_string(),
            });
        }

        Ok(exports)
    }

    async fn import_calls_with_mapping(
        &self,
        calls_data: &Value,
        address_to_uid: &HashMap<String, String>,
    ) -> Result<i64> {
        let calls_array = calls_data
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("calls must be an array"))?;

        let mut call_count = 0i64;
        let mut skipped_count = 0i64;

        for call_data in calls_array {
            let from_addr = call_data
                .get("from_address")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Call missing from_address"))?;

            let to_addr = call_data
                .get("to_address")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Call missing to_address"))?;

            let offset = call_data
                .get("offset")
                .and_then(|v| v.as_str())
                .unwrap_or("0x0");

            let call_type_str = call_data
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("direct");

            let call_type = CallType::from_str(call_type_str).unwrap_or(CallType::Direct);

            let from_normalized =
                normalize_address(from_addr).unwrap_or_else(|| from_addr.to_string());
            let to_normalized = normalize_address(to_addr).unwrap_or_else(|| to_addr.to_string());

            let from_uid = address_to_uid
                .get(&from_normalized)
                .or_else(|| address_to_uid.get(from_addr));
            let to_uid = address_to_uid
                .get(&to_normalized)
                .or_else(|| address_to_uid.get(to_addr));

            if let (Some(from_uid), Some(to_uid)) = (from_uid, to_uid) {
                let calls = Calls::new(offset.to_string(), call_type);
                self.importer
                    .create_calls_relationship(&calls, from_uid, to_uid)
                    .await?;
                call_count += 1;
            } else {
                skipped_count += 1;
            }
        }

        if skipped_count > 0 {
            eprintln!(
                "[WARN] Skipped {} call relationships due to unresolved addresses",
                skipped_count
            );
        }

        Ok(call_count)
    }

    pub async fn query_functions(
        &self,
        pattern: &str,
        binary: Option<&str>,
    ) -> Result<Vec<Function>> {
        self.importer.query_functions(pattern, binary).await
    }

    pub async fn query_binary_info(&self, binary_name: &str) -> Result<Option<Binary>> {
        self.importer.query_binary_info(binary_name).await
    }

    pub async fn query_callgraph_with_depth(
        &self,
        function_name: &str,
        binary: Option<&str>,
        max_depth: usize,
    ) -> Result<CallGraph> {
        self.importer
            .query_callgraph_with_depth(function_name, binary, max_depth)
            .await
    }

    pub async fn query_xrefs(&self, address: &str, binary: Option<&str>) -> Result<Vec<Xref>> {
        self.importer.query_xrefs(address, binary).await
    }

    pub fn importer(&self) -> &crate::neo4j::GraphImporter {
        &self.importer
    }
}
