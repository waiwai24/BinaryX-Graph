use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainsRelationship {
    pub from_uid: String,
    pub to_uid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallsRelationship {
    pub from_uid: String,
    pub to_uid: String,
    pub call_site: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CallType {
    /// Direct function call
    Direct,
    /// Indirect function call
    Indirect,
    /// Virtual function call
    Virtual,
    /// Tail call optimization
    Tail,
}

impl FromStr for CallType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "direct" => Ok(CallType::Direct),
            "indirect" => Ok(CallType::Indirect),
            "virtual" => Ok(CallType::Virtual),
            "tail" => Ok(CallType::Tail),
            _ => Ok(CallType::Direct), // Default to Direct if unknown
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RefType {
    /// Memory read operation
    Read,
    /// Load Effective Address operation
    Lea,
    /// Push operation
    Push,
    /// Data transfer operation
    Mov,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contains {
    /// Relationship type, fixed as "CONTAINS"
    #[serde(rename = "type")]
    pub rel_type: String,
}

impl Contains {
    pub fn new() -> Self {
        Self {
            rel_type: "CONTAINS".to_string(),
        }
    }
}

impl Default for Contains {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Imports {
    /// Relationship type, fixed as "IMPORTS"
    #[serde(rename = "type")]
    pub rel_type: String,
}

impl Imports {
    pub fn new() -> Self {
        Self {
            rel_type: "IMPORTS".to_string(),
        }
    }
}

impl Default for Imports {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calls {
    /// Relationship type, fixed as "CALLS"
    #[serde(rename = "type")]
    pub rel_type: String,
    /// Offset address of the call instruction (hexadecimal format)
    pub offset: String,
    /// Call type
    pub call_type: CallType,
}

impl Calls {
    pub fn new(offset: String, call_type: CallType) -> Self {
        Self {
            rel_type: "CALLS".to_string(),
            offset,
            call_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvesTo {
    /// Relationship type, fixed as "RESOLVES_TO"
    #[serde(rename = "type")]
    pub rel_type: String,
}

impl ResolvesTo {
    pub fn new() -> Self {
        Self {
            rel_type: "RESOLVES_TO".to_string(),
        }
    }
}

impl Default for ResolvesTo {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct References {
    /// Relationship type, fixed as "REFERENCES"
    #[serde(rename = "type")]
    pub rel_type: String,
    /// Offset address of the reference instruction (hexadecimal format)
    pub offset: String,
    /// Reference type
    pub ref_type: RefType,
}

impl References {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasString {
    /// Relationship type, fixed as "HAS_STRING"
    #[serde(rename = "type")]
    pub rel_type: String,
}

impl HasString {
    pub fn new() -> Self {
        Self {
            rel_type: "HAS_STRING".to_string(),
        }
    }
}

impl Default for HasString {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BelongsTo {
    /// Relationship type, fixed as "BELONGS_TO"
    #[serde(rename = "type")]
    pub rel_type: String,
}

impl BelongsTo {
    pub fn new() -> Self {
        Self {
            rel_type: "BELONGS_TO".to_string(),
        }
    }
}

impl Default for BelongsTo {
    fn default() -> Self {
        Self::new()
    }
}
