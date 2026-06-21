---
name: Use FalkorDB parameterized queries
description: Prefix read queries with CYPHER parameters for plan caching and safer value handling
---

# Use FalkorDB parameterized queries

Pass user-supplied values as parameters so FalkorDB can cache and reuse the query plan.

## Usage

Prefix the query with `CYPHER` and `name=value` declarations, then reference each value with `$name`
inside the query body.

## Example

```cypher
CYPHER name='Alice'
MATCH (u:User {name: $name})
RETURN u.id
```

## Notes

- Parameters let FalkorDB cache and reuse query execution plans, avoiding repeated parsing and planning.
- Declare values after the `CYPHER` keyword using `name=value`; reference them as `$name`.
- Prefer parameters for user-supplied values; this also avoids query-injection issues.
- Parameterization does not change the result shape; it only affects planning and safety.
