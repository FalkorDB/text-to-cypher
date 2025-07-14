use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub use super::attribute::Attribute;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Entity {
    pub label: String,
	#[serde(skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<Attribute>,
	#[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Entity {
    pub const fn new(
        label: String,
        attributes: Vec<Attribute>,
        description: Option<String>,
    ) -> Self {
        Self {
            label,
            attributes,
            description,
        }
    }
}

impl std::fmt::Display for Entity {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let description = self.description.as_ref().map_or("None", |d| d.as_str());
        write!(
            f,
            "Entity: {} ({} attributes, description: {})",
            self.label,
            self.attributes.len(),
            description
        )
    }
}
