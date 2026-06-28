---
name: Find paths in FalkorDB
description: Use algo.SPpaths/algo.SSpaths procedures and variable-length patterns to find paths in read queries
---

# Find paths in FalkorDB

Traverse relationships with variable-length patterns and FalkorDB's path-finding functions and procedures.

## Usage

Use the `algo.SPpaths` procedure to find the shortest path between two specific nodes (single pair),
and `algo.SSpaths` for shortest paths from one source to all reachable nodes. Use variable-length
relationship patterns with `shortestPath`/`allShortestPaths` only for simple unweighted pattern matching.

## Example

```cypher
MATCH (a:City {name: 'Paris'}), (b:City {name: 'Berlin'})
CALL algo.SPpaths({sourceNode: a, targetNode: b, relTypes: ['ROAD'], relDirection: 'outgoing', pathCount: 1})
YIELD path
RETURN [n IN nodes(path) | n.name] AS route
```

## Notes

- Prefer `algo.SPpaths()` for "shortest path between X and Y" questions; it is FalkorDB's dedicated
  single-pair shortest-path procedure.
- Parameters: `sourceNode`, `targetNode`, `relTypes` (array), `relDirection` (`outgoing` | `incoming` | `both`),
  `pathCount` (1 = single shortest, 0 = all shortest, n = up to n paths).
- Add `weightProp` (e.g. `'dist'`, `'time'`) to minimize a weighted property, and `costProp`/`maxCost`
  to constrain total cost. It `YIELD`s `path`, `pathWeight`, and `pathCost`.
- Use `algo.SSpaths()` (single source) for all shortest paths from one node to all reachable destinations.
- Variable-length paths use `-[:TYPE*minHops..maxHops]->`; bound the hops to avoid expensive traversals.
- `shortestPath(...)`/`allShortestPaths(...)` are unweighted pattern helpers for simple cases.
- Use `-[:TYPE]-` for undirected traversal and `<-[:TYPE]-` to follow relationships in reverse.
- These are all read-only traversals.
