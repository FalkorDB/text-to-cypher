use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, strum::Display)]
pub enum AttributeType {
    String,
    Number,
    Integer,
    Float,
    Boolean,
    DateTime,
    List,
    Map,
    Vector,
    Point,
}

impl std::str::FromStr for AttributeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Match exact strings returned by FalkorDB's typeof() function
        match s {
            "String" => Ok(Self::String),
            "Integer" => Ok(Self::Integer),
            "Float" => Ok(Self::Float),
            "Boolean" => Ok(Self::Boolean),
            "List" => Ok(Self::List),
            "Map" => Ok(Self::Map),
            "Point" => Ok(Self::Point),
            "Vectorf32" => Ok(Self::Vector),
            _ => Err(format!("Unknown FalkorDB type: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Attribute {
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: AttributeType,
    #[serde(skip_serializing)]
    pub count: i64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub unique: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<String>>,
}

impl Attribute {
    #[must_use]
    pub const fn new(
        name: String,
        r#type: AttributeType,
        count: i64,
        unique: bool,
        required: bool,
    ) -> Self {
        Self {
            name,
            r#type,
            count,
            unique,
            required,
            examples: None,
        }
    }

    /// Create a new Attribute with example values
    #[must_use]
    #[allow(dead_code)]
    pub const fn with_examples(
        name: String,
        r#type: AttributeType,
        count: i64,
        unique: bool,
        required: bool,
        examples: Option<Vec<String>>,
    ) -> Self {
        Self {
            name,
            r#type,
            count,
            unique,
            required,
            examples,
        }
    }
}

impl std::fmt::Display for Attribute {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "{}: {} (count: {}, unique: {}, required: {})",
            self.name, self.r#type, self.count, self.unique, self.required
        )
    }
}
