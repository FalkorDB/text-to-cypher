---
name: Query FalkorDB vector indexes
description: Find nearest-neighbor nodes with the db.idx.vector.queryNodes procedure when a vector index exists
---

# Query FalkorDB vector indexes

Use FalkorDB's vector search procedure to find approximate nearest neighbors by embedding similarity.

## Usage

Only when the ontology declares a vector index for the label and property, call
`db.idx.vector.queryNodes('Label', 'property', k, vecf32([...]))` and yield `node, score`.

## Example

```cypher
CALL db.idx.vector.queryNodes('Product', 'embedding', 5, vecf32([0.1, 0.2, 0.3])) YIELD node, score
RETURN node.name, score
```

## Notes

- `k` is the number of nearest neighbors to return; results are ordered by similarity.
- Pass the query vector with `vecf32([...])`.
- `YIELD node, score` exposes each match and its similarity score.
- Use this only when the ontology lists a vector index for the label and property.
- Vector search is read-only; it queries an existing index and does not modify the graph.
