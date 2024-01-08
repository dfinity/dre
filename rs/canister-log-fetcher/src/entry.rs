use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Entry {
    pub file: String,
    pub line: u64,
    pub message: String,
    pub timestamp: u64,
    // Optional and situational fields
    pub counter: Option<u64>,
    pub priority: Option<String>,
    pub severity: Option<String>,
}

impl TryFrom<&serde_json::Value> for Entry {
    type Error = String;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        let parsed = Self {
            file: value["file"]
                .as_str()
                .ok_or(format!("Couldn't parse 'file' from the value {:?}", value))?
                .to_string(),
            line: value["line"]
                .as_u64()
                .ok_or(format!("Couldn't parse 'line' for entry {:?}", value))?,
            message: value["message"]
                .as_str()
                .ok_or(format!("Couldn't parse 'message' from value {:?}", value))?
                .to_string(),
            timestamp: value["timestamp"]
                .as_u64()
                .ok_or(format!("Couldn't parse 'timestamp' from value {:?}", value))?,
            counter: value["counter"].as_u64(),
            priority: value["priority"].as_str().map(|s| s.to_string()),
            severity: value["severity"].as_str().map(|s| s.to_string()),
        };
        Ok(parsed)
    }
}
