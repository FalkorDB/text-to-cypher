use std::vec;

use falkordb::{AsyncGraph, FalkorDBError, FalkorValue};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::schema::{
    attribute::{Attribute, AttributeType},
    entity::Entity,
    relation::Relation,
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Schema {
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
}

impl std::fmt::Display for Schema {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "Schema with {} entities and {} relations",
            self.entities.len(),
            self.relations.len()
        )
    }
}

impl Schema {
    const fn empty() -> Self {
        Self {
            entities: Vec::new(),
            relations: Vec::new(),
        }
    }

    pub fn add_entity(
        &mut self,
        entity: Entity,
    ) {
        self.entities.push(entity);
    }

    pub fn add_relation(
        &mut self,
        relation: Relation,
    ) {
        self.relations.push(relation);
    }

    async fn collect_entity_attributes(
        graph: &mut AsyncGraph,
        label: &str,
        sample_size: usize,
    ) -> Result<Vec<Attribute>, FalkorDBError> {
        let query = format!(
            r"
            MATCH (a:{label})
            CALL {{
                WITH a
                RETURN [k IN keys(a) | [k, typeof(a[k])]] AS types
            }}
            WITH types
            LIMIT {sample_size}
            UNWIND types AS kt
            RETURN kt, count(1)
            ORDER BY kt[0]
            "
        );

        Self::collect_attributes(graph, label, &query).await
    }

    async fn collect_relationship_attributes(
        graph: &mut AsyncGraph,
        label: &str,
        sample_size: usize,
    ) -> Result<Vec<Attribute>, FalkorDBError> {
        let query = format!(
            r"
            MATCH ()-[a:{label}]->()
            CALL {{
                WITH a
                RETURN [k IN keys(a) | [k, typeof(a[k])]] AS types
            }}
            WITH types
            LIMIT {sample_size}
            UNWIND types AS kt
            RETURN kt, count(1)
            ORDER BY kt[0]
            "
        );

        Self::collect_attributes(graph, label, &query).await
    }

    async fn collect_attributes(
        graph: &mut AsyncGraph,
        label: &str,
        query: &str,
    ) -> Result<Vec<Attribute>, FalkorDBError> {
        tracing::info!("Collecting attributes for label '{}': {}", label, query);

        let entity_attributes = graph.ro_query(query).execute().await?;
        let mut attributes = Vec::new();

        for record in entity_attributes.data {
            // Extract both kt (key-type info) and count from the record
            if let (Some(FalkorValue::Array(kt_array)), Some(FalkorValue::I64(count))) = (record.first(), record.get(1))
            {
                // kt_array should contain [key_name, type_name]
                if kt_array.len() >= 2 {
                    if let (Some(FalkorValue::String(key_name)), Some(FalkorValue::String(type_name))) =
                        (kt_array.first(), kt_array.get(1))
                    {
                        tracing::info!("Found attribute: key={}, type={}, count={}", key_name, type_name, count);

                        // Parse the type_name to AttributeType
                        let attr_type = type_name.parse::<AttributeType>().unwrap_or_else(|_| {
                            tracing::warn!("Unknown attribute type '{}', defaulting to String", type_name);
                            AttributeType::String
                        });

                        attributes.push(Attribute::new(key_name.clone(), attr_type, *count, false, false));
                    }
                }
            }
        }

        Ok(attributes)
    }

    async fn get_entity_labels(graph: &mut AsyncGraph) -> Result<Vec<String>, FalkorDBError> {
        // Get node labels (entity types)
        let labels_result = graph.ro_query("CALL db.labels()").execute().await?;

        // Collect labels first to avoid borrowing issues
        let mut entity_labels = Vec::new();
        for record in labels_result.data {
            if let Some(FalkorValue::String(label)) = record.first() {
                entity_labels.push(label.clone());
            }
        }

        Ok(entity_labels)
    }

    async fn get_relationship_labels(graph: &mut AsyncGraph) -> Result<Vec<String>, FalkorDBError> {
        let relations_result = graph.ro_query("CALL db.relationshipTypes()").execute().await?;

        let mut relationship_labels = Vec::new();
        for record in relations_result.data {
            if let Some(FalkorValue::String(relation_label)) = record.first() {
                relationship_labels.push(relation_label.clone());
            }
        }

        Ok(relationship_labels)
    }

    async fn get_relationship_attributes(
        graph: &mut AsyncGraph,
        relationship_labels: &[String],
        sample_size: usize,
    ) -> Result<Vec<(String, Vec<Attribute>)>, FalkorDBError> {
        let mut relationship_attributes = vec![];

        for relationship_label in relationship_labels {
            let attributes = Self::collect_relationship_attributes(graph, relationship_label, sample_size).await?;
            relationship_attributes.push((relationship_label.to_owned(), attributes));
        }

        Ok(relationship_attributes)
    }

    /// Discover the schema from a graph database.
    ///
    /// # Errors
    ///
    /// Returns an error if the graph operations fail.
    pub async fn discover_from_graph(
        graph: &mut AsyncGraph,
        sample_size: usize,
    ) -> Result<Self, FalkorDBError> {
        let mut schema: Self = Self::empty();

        let entity_labels = Self::get_entity_labels(graph).await?;

        for entity_label in &entity_labels {
            let attributes = Self::collect_entity_attributes(graph, entity_label, sample_size).await?;
            schema.add_entity(Entity::new(entity_label.to_owned(), attributes, None));
        }

        // Get relationship types
        let relationship_labels = Self::get_relationship_labels(graph).await?;

        let relationship_attributes =
            Self::get_relationship_attributes(graph, &relationship_labels, sample_size).await?;

        let entities = schema.entities.clone();
        for (label, attributes) in &relationship_attributes {
            for source_entity in &entities {
                for target_entity in &entities {
                    tracing::info!(
                        "Processing relationships from {} to {}",
                        source_entity.label,
                        target_entity.label
                    );
                    let query = format!(
                        "MATCH (s:{})-[a:{label}]->(t:{}) return a limit 1",
                        source_entity.label, target_entity.label
                    );
                    let query_result = graph.ro_query(&query).execute().await?;
                    if !query_result.data.is_empty() {
                        let relation = Relation::new(
                            label.to_owned(),
                            source_entity.label.clone(),
                            target_entity.label.clone(),
                            attributes.to_owned(),
                        );
                        schema.add_relation(relation);
                    }
                }
            }
        }

        Ok(schema)
    }
}
