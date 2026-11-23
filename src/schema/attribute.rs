use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, strum::EnumString, strum::Display)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub enum AttributeType {
    String,
    Number,
    Integer,
    Float,
    Boolean,
    DateTime,
    Date,
    Time,
    Duration,
    List,
    Map,
    Vector,
    Point,
}

impl AttributeType {
    /// Maps a type name returned by `FalkorDB`'s `typeof()` to an [`AttributeType`].
    ///
    /// `FalkorDB` reports some types under names that differ from the variant names
    /// (`Datetime`, `Vectorf32`, ...), so this accepts both spellings. Returns `None`
    /// for names it does not recognise.
    #[must_use]
    pub fn from_falkordb_type(type_name: &str) -> Option<Self> {
        match type_name {
            "String" => Some(Self::String),
            "Number" => Some(Self::Number),
            "Integer" => Some(Self::Integer),
            "Float" => Some(Self::Float),
            "Boolean" => Some(Self::Boolean),
            "Datetime" | "DateTime" => Some(Self::DateTime),
            "Date" => Some(Self::Date),
            "Time" => Some(Self::Time),
            "Duration" => Some(Self::Duration),
            "List" => Some(Self::List),
            "Map" => Some(Self::Map),
            "Vectorf32" | "Vector" => Some(Self::Vector),
            "Point" => Some(Self::Point),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(ToSchema))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_falkordb_typeof_names() {
        // Names as returned by `typeof()` on FalkorDB 4.22
        let cases = [
            ("String", "String"),
            ("Integer", "Integer"),
            ("Float", "Float"),
            ("Boolean", "Boolean"),
            ("Datetime", "DateTime"),
            ("Date", "Date"),
            ("Time", "Time"),
            ("Duration", "Duration"),
            ("Vectorf32", "Vector"),
            ("Point", "Point"),
            ("List", "List"),
            ("Map", "Map"),
        ];
        for (falkordb_name, expected) in cases {
            let parsed = AttributeType::from_falkordb_type(falkordb_name)
                .unwrap_or_else(|| panic!("{falkordb_name} should be recognised"));
            assert_eq!(parsed.to_string(), expected, "typeof name {falkordb_name}");
        }
    }

    #[test]
    fn unknown_type_is_none() {
        assert!(AttributeType::from_falkordb_type("SomethingNew").is_none());
    }
}
