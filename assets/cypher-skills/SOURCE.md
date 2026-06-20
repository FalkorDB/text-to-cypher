# Built-in FalkorDB Cypher skills — provenance

These skills are **derived** (not copied verbatim) from FalkorDB's skills repository and the FalkorDB
documentation, then adapted for read-only natural-language-to-Cypher generation:

- Source: https://github.com/FalkorDB/skills (`cypher-skills/`), ref
  `172978316e493c48ca352a0be6fb668a9f728855` (the same ref baked into the Docker image's `CYPHER_SKILLS_REF`).
- Reference docs: https://docs.falkordb.com

## Curation rules (scope: read-only awareness, issue #82 option A)
- **Pure Cypher only** — upstream examples use `redis-cli GRAPH.QUERY ...`; these are rewritten as the
  bare Cypher the model should emit.
- **Read-only only** — DDL/write skills are excluded (range/constraint creation, node/relationship
  creation, property updates, MERGE). Index *creation* is out of scope; index *querying* is included.
- **No operational/admin procedures** — `GRAPH.EXPLAIN`/`PROFILE`/`SLOWLOG`/memory inspection are not
  emitted by the generator; only their query-design guidance is folded into the always-on reference.

These files are embedded at compile time (see `src/skills/builtin.rs`) so every consumer — library, napi
bindings, browser, and server — gets `FalkorDB`-specific context by default, not just the Docker image.
The content is hand-curated/derived (not a verbatim copy), so update it deliberately and bump the `ref`
above when re-deriving from upstream.
