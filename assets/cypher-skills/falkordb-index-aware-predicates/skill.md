---
name: Apply FalkorDB index-aware predicates
description: Write predicates that let FalkorDB use indexes and avoid full scans in read queries
---

# Apply FalkorDB index-aware predicates

Design `WHERE` predicates so FalkorDB can use indexes instead of scanning every node.

## Usage

Apply equality or range predicates directly to indexed properties. Avoid not-equal filters and avoid
wrapping the indexed property in a function.

## Example

```cypher
MATCH (p:Person)
WHERE p.age >= 30 AND p.age < 40
RETURN p.name, p.age
```

## Notes

- Not-equal (`<>` / `!=`) predicates are not index-accelerated and force a full scan; use them only when
  exclusion is explicitly required.
- Equality (`=`) and range (`<`, `<=`, `>`, `>=`) predicates on indexed properties can use an index scan.
- Applying a function to the indexed property (e.g. `toLower(p.name) = 'alice'`) prevents index use; keep
  the indexed property bare on one side of the predicate when an index exists.
- Prefer positive predicates that preserve the question's intent over negations.
