---
name: Query FalkorDB full-text indexes
description: Search text properties with the db.idx.fulltext.queryNodes procedure when a full-text index exists
---

# Query FalkorDB full-text indexes

Use FalkorDB's full-text search procedure to match text properties with wildcard and fuzzy matching.

## Usage

Only when the ontology declares a full-text index for the label, call
`db.idx.fulltext.queryNodes('Label', 'search_term')` and yield `node`.

## Example

```cypher
CALL db.idx.fulltext.queryNodes('Movie', 'Jun*') YIELD node
RETURN node.title
```

## Notes

- The search term supports wildcards (e.g. `'Jun*'`) and fuzzy matching.
- `YIELD node` exposes each matched node; project specific properties in `RETURN`.
- Use this only when the ontology lists a full-text index for the label; otherwise use a normal
  `WHERE ... CONTAINS ...` predicate.
- Full-text search is read-only; it queries an existing index and does not modify the graph.
