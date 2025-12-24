use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
