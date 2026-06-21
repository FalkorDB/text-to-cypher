---
name: Find paths in FalkorDB
description: Use variable-length patterns, shortest-path functions, and weighted path procedures in read queries
---

# Find paths in FalkorDB

Traverse relationships with variable-length patterns and FalkorDB's path-finding functions and procedures.

## Usage

Use variable-length relationship patterns and `shortestPath`/`allShortestPaths` for unweighted paths, and
the `algo.SPpaths`/`algo.SSpaths` procedures for weighted paths.

## Example

```cypher
MATCH path = allShortestPaths((a:City {name: 'Paris'})-[:ROAD*]->(b:City {name: 'Berlin'}))
RETURN [n IN nodes(path) | n.name] AS route
```

## Notes

- Variable-length paths use `-[:TYPE*minHops..maxHops]->`; bound the hops to avoid expensive traversals.
- `shortestPath(...)` returns one shortest path; `allShortestPaths(...)` returns every shortest path.
- For weighted shortest paths use the procedures `algo.SPpaths()` (single pair) and `algo.SSpaths()`
  (single source).
- Use `-[:TYPE]-` for undirected traversal and `<-[:TYPE]-` to follow relationships in reverse.
- These are all read-only traversals.
