# Copilot Instructions for text-to-cypher

## Build, Lint, and Test

```bash
# Build (full server, the default)
cargo build --release

# Build library-only (no REST/MCP server deps)
cargo build --lib --no-default-features

# Lint — CI uses pedantic + nursery and treats warnings as errors
cargo clippy -- -W clippy::pedantic -W clippy::nursery -D warnings

# Format check (uses rustfmt.toml: edition 2024, max_width 120)
cargo fmt -- --check

# Run all tests
cargo test

# Run a single test by name
cargo test test_client_creation

# Run tests in a specific module
cargo test --lib core::tests
```

The binary (`src/main.rs`) requires the `server` feature (enabled by default). Library-only consumers use `default-features = false`.

## Architecture

### Processing Pipeline

Natural language → **schema discovery** → **Cypher generation** (AI) → **validation** → **execution** → **answer generation** (AI)

Key flow through modules:

1. **`src/core.rs`** — Core pipeline logic. Orchestrates: `discover_graph_schema` → `generate_cypher_query` → `execute_cypher_query` → `generate_final_answer`.
2. **`src/processor.rs`** — Non-streaming request/response wrapper around `core` for library use.
3. **`src/main.rs`** — Standalone actix-web server with SSE streaming. Defines `send!` / `send_option!` / `send_result!` macros for SSE event dispatch.

### AI Integration

Uses the **`genai`** crate for multi-provider LLM support (OpenAI, Anthropic, Gemini, Groq, Cohere, Ollama). Model names are prefixed by provider (e.g., `anthropic:claude-3`). A custom `AuthResolver` injects API keys at runtime.

### Template System

`src/template.rs` renders prompts from `templates/*.txt` files using `{{VARIABLE}}` placeholders embedded at compile time via `include_str!`. Three templates:
- `system_prompt.txt` — instructs the LLM to generate OpenCypher from an ontology
- `user_prompt.txt` — wraps the user question
- `last_request_prompt.txt` — asks the LLM to produce a natural-language answer from query results

### Schema Discovery

`src/schema/` discovers FalkorDB graph schemas automatically — entities, relations, attributes with types and example values. The schema is serialized to JSON and injected into the system prompt as the ontology.

### MCP Server

`src/mcp/` implements a Model Context Protocol server (port 3001) exposing a `talk_with_a_graph` tool and graph resources at `falkordb://graph/{name}`. Built with `rust-mcp-sdk`.

### Query Safety

`src/validator.rs` validates generated Cypher before execution — checks for valid syntax patterns and rejects dangerous operations (DROP, DELETE).

### FalkorDB Cypher Skills

The system prompt (`templates/system_prompt.txt`) incorporates FalkorDB-specific Cypher best practices derived from the [FalkorDB/skills](https://github.com/FalkorDB/skills/tree/main/cypher-skills) repository. These include read-only constraints, index-awareness rules (e.g., `<>`/`!=` not index-accelerated), full-text and vector search syntax, and parameterized query support. Full-text and vector search guidance is conditional — it only applies when the ontology explicitly declares those indexes (schema discovery does not currently expose index metadata). When updating the system prompt, refer to the skills repo for authoritative FalkorDB Cypher guidance.

### Dynamic Skill Loading

`src/skills/` implements a two-tier dynamic skill loading system:

- **Tier 1 (catalog)**: Skill names and descriptions are injected into the system prompt via the `{{SKILLS_CATALOG}}` placeholder. This gives the LLM awareness of available skills without bloating the context.
- **Tier 2 (tool calling)**: For providers that support tool calling (OpenAI, Anthropic, Gemini, xAI, DeepSeek), the LLM can request full skill content on-demand via the `read_skill` tool. Providers without tool support (Groq, Ollama, Cohere) get all skill content injected directly into the prompt as a fallback.

Skills are loaded from the `SKILLS_DIR` environment variable at startup. Expected directory structure:
```text
skills_dir/
  skill-name/
    skill.md     # YAML frontmatter (name, description) + markdown body
```

Key modules:
- `src/skills/parser.rs` — Parses `skill.md` files (YAML frontmatter via `serde_yaml` + markdown body)
- `src/skills/loader.rs` — Scans directories recursively for `skill.md` files, uses directory name as stable skill ID
- `src/skills/mod.rs` — `SkillCatalog` (catalog rendering, tool definition, skill lookup), `supports_tool_calling()` provider gating

### Downstream Consumers

This library is consumed by [FalkorDB/text-to-cypher-node](https://github.com/FalkorDB/text-to-cypher-node) (Node.js bindings), which in turn powers [FalkorDB/falkordb-browser](https://github.com/FalkorDB/falkordb-browser).

## Conventions

- **Feature gating**: Server-only code is behind `#[cfg(feature = "server")]`. Schema types use `#[cfg_attr(feature = "server", derive(ToSchema))]` for conditional OpenAPI derive.
- **unsafe_code is forbidden** (`lints.rust.unsafe_code = "forbid"` in Cargo.toml).
- **Error pattern**: Functions return `Result<T, Box<dyn Error + Send + Sync>>`. The `ApiError` enum in `error.rs` maps genai errors to HTTP status codes with provider-aware messages.
- **Formatting**: Rust edition 2024, `fn_params_layout = "Vertical"`, `max_width = 120`.
- **`#[must_use]`** is applied to pure functions returning constructed values (builders, renderers).
- **Regex patterns** use `OnceLock` for lazy-static initialization (see `validator.rs`, `main.rs`).
- **Environment config**: `.env` file with `DEFAULT_MODEL`, `DEFAULT_KEY`, `FALKORDB_CONNECTION`, `REST_PORT` (8080), `MCP_PORT` (3001), `SKILLS_DIR` (optional, path to skill files).

## Workflow

- **Rubber duck review before pushing**: Always run a rubber duck review on your changes before creating commits or pull requests. This catches contradictions, regressions, and blind spots early.

## Deployment Targets

- **Standalone Docker**: All-in-one container (FalkorDB + web UI + API + MCP + Cypher skills) via `supervisord.conf`. Skills are baked in from [FalkorDB/skills](https://github.com/FalkorDB/skills). Pin with `--build-arg SKILLS_REF=<tag>`.
- **Rust library**: `default-features = false` for embedding in other Rust apps
- **Cross-compilation**: ARM64 via `cross` tool (`Cross.toml`)

## Development with `just`

A `justfile` provides common development recipes. Install [just](https://github.com/casey/just) then:

```bash
just download-skills   # Fetch FalkorDB Cypher skills
just check             # lint + test (CI equivalent)
just dev               # Run server with skills (auto-downloads if missing)
just list-skills       # Show loaded skills
```
