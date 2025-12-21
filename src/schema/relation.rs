use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use utoipa::ToSchema;

use crate::schema::entity::Attribute;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct Relation {
    pub label: String,
    pub source: String,
    pub target: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<Attribute>,
}

impl Relation {
    #[must_use]
    pub const fn new(
        label: String,
        source: String,
        target: String,
        attributes: Vec<Attribute>,
    ) -> Self {
        Self {
            label,
            source,
            target,
            attributes,
        }
    }
}

impl std::fmt::Display for Relation {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "Relation: {} ({} -> {}, {} attributes)",
            self.label,
            self.source,
            self.target,
            self.attributes.len()
        )
    }
}
