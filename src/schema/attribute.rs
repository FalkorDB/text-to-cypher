use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, strum::EnumString, strum::Display)]
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
