use crate::utils::uid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BinaryFormat {
    /// Windows PE (Portable Executable) file format
    PE,
    /// Linux/Unix ELF (Executable and Linkable Format) file format
    Elf,
    /// macOS Mach-O file format
    MachO,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FunctionType {
    /// Functions defined internally in the binary file
    Internal,
    /// Functions imported from external libraries
    Import,
    /// Functions exported from the binary file for use by other modules
    Export,
    /// Jump functions used for indirect calls or delayed binding
    Thunk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binary {
    /// Hash of the binary file, typically SHA-256
    pub hash: std::string::String,
    /// File name of the binary file
    pub filename: std::string::String,
    /// Full path to the binary file
    pub file_path: std::string::String,
    /// Size of the binary file in bytes
    pub file_size: u64,
    /// Format type of the binary file
    pub format: BinaryFormat,
    /// Target architecture of the binary file
    pub arch: std::string::String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    /// Unique identifier for the function
    pub uid: std::string::String,
    /// Function name
    pub name: std::string::String,
    /// Function type
    pub r#type: FunctionType,
    /// Function address (hexadecimal format), may be None for imported functions
    pub address: Option<std::string::String>,
    /// Size of the function in bytes
    pub size: Option<u64>,
}

impl Function {
    pub fn create_internal(binary_hash: &str, address: u64, name: &str, is_export: bool) -> Self {
        let hex_addr = format!("0x{address:x}");
        Self {
            uid: format!("{binary_hash}:{hex_addr}"),
            name: name.to_string(),
            r#type: if is_export {
                FunctionType::Export
            } else {
                FunctionType::Internal
            },
            address: Some(hex_addr),
            size: None,
        }
    }

    pub fn create_import(library: &str, name: &str) -> Self {
        let lib_normalized = library.to_lowercase();
        Self {
            // Global UID for imported APIs, shared across binaries.
            uid: format!("imp:{lib_normalized}:{name}"),
            name: name.to_string(),
            r#type: FunctionType::Import,
            // Import address is binary-specific; store it on the Binary-[:CONTAINS] edge instead.
            address: None,
            size: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringNode {
    /// Content of the string
    pub value: String,
    /// Unique identifier of the string, generated based on content hash
    pub uid: String,
}

impl StringNode {
    pub fn new(value: String) -> Self {
        // Reduce duplicates caused by extractor variations (e.g. trailing NUL).
        let normalized_value = value.trim_end_matches('\0').to_string();
        let uid = uid::generate_string_uid(&normalized_value);
        Self {
            value: normalized_value,
            uid,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringOccurrence {
    /// UID of the referenced String node
    pub string_uid: String,
    /// Address where the string is located in the binary (hexadecimal format)
    pub address: Option<String>,
}

impl StringOccurrence {
    pub fn new(string_uid: String, address: Option<String>) -> Self {
        Self {
            string_uid,
            address,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringSearchHit {
    pub uid: String,
    pub value: String,
    pub score: f64,
    pub sample_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    /// Library name, uniformly converted to lowercase
    pub name: std::string::String,
}

impl Library {
    pub fn create(name: &str) -> Self {
        Self {
            name: name.to_lowercase(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    /// Name of the imported symbol
    pub name: std::string::String,
    /// Address in the Import Address Table (hexadecimal format)
    pub address: std::string::String,
    /// Name of the library from which the symbol is imported
    pub library: std::string::String,
}

impl Import {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Export {
    /// Name of the exported symbol
    pub name: std::string::String,
    /// Address of the exported symbol (hexadecimal format)
    pub address: std::string::String,
}

impl Export {}
