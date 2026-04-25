# text-to-cypher development tasks
# Install just: https://github.com/casey/just#installation

# Default recipe: show available commands
default:
    @just --list

# ── Skills ────────────────────────────────────────────────────────────────────

skills_dir := "skills"
skills_repo := "https://github.com/FalkorDB/skills"
skills_ref := "main"

# Download FalkorDB Cypher skills from the skills repository
download-skills:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "📥 Downloading FalkorDB Cypher skills (ref: {{skills_ref}})..."
    rm -rf "{{skills_dir}}"
    tmpdir=$(mktemp -d)
    curl -sL "{{skills_repo}}/archive/{{skills_ref}}.tar.gz" | tar -xz -C "$tmpdir" --strip-components=1
    mv "$tmpdir/cypher-skills" "{{skills_dir}}"
    rm -rf "$tmpdir"
    count=$(ls "{{skills_dir}}" | wc -l | tr -d ' ')
    echo "✅ Downloaded ${count} skills to ./{{skills_dir}}/"

# Download skills pinned to a specific ref (branch, tag, or commit)
download-skills-pinned ref:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "📥 Downloading FalkorDB Cypher skills (ref: {{ref}})..."
    rm -rf "{{skills_dir}}"
    tmpdir=$(mktemp -d)
    curl -sL "{{skills_repo}}/archive/{{ref}}.tar.gz" | tar -xz -C "$tmpdir" --strip-components=1
    mv "$tmpdir/cypher-skills" "{{skills_dir}}"
    rm -rf "$tmpdir"
    count=$(ls "{{skills_dir}}" | wc -l | tr -d ' ')
    echo "✅ Downloaded ${count} skills to ./{{skills_dir}}/"

# ── Build ─────────────────────────────────────────────────────────────────────

# Build in debug mode (fast compilation)
build:
    cargo build

# Build in release mode (optimized)
build-release:
    cargo build --release

# Build library only (no server dependencies)
build-lib:
    cargo build --lib --no-default-features

# ── Quality ───────────────────────────────────────────────────────────────────

# Check formatting
fmt:
    cargo fmt -- --check

# Run clippy with CI-level strictness
clippy:
    cargo clippy -- -W clippy::pedantic -W clippy::nursery -D warnings

# Run all lints (fmt + clippy)
lint: fmt clippy

# Run all tests
test:
    cargo test

# Full CI check: lint + test
check: lint test

# ── Run ───────────────────────────────────────────────────────────────────────

# Run the server in development mode (downloads skills if missing)
dev: _ensure-skills
    SKILLS_DIR=./{{skills_dir}} cargo run

# Run the server in release mode with skills
run: _ensure-skills
    SKILLS_DIR=./{{skills_dir}} cargo run --release

# ── Docker ────────────────────────────────────────────────────────────────────

# Build Docker image locally
docker-build version="v0.1.0-alpha.1":
    ./docker-build.sh --version {{version}} --local

# Build Docker image and push to registry
docker-push version registry:
    ./docker-build.sh --version {{version}} --registry {{registry}} --push

# ── Utilities ─────────────────────────────────────────────────────────────────

# Clean build artifacts
clean:
    cargo clean
    rm -rf skills/

# Show loaded skills
list-skills: _ensure-skills
    #!/usr/bin/env bash
    echo "📚 Skills in ./{{skills_dir}}/:"
    for dir in {{skills_dir}}/*/; do
        if [ -f "${dir}skill.md" ]; then
            name=$(head -5 "${dir}skill.md" | grep "^name:" | sed 's/name: *//')
            desc=$(head -5 "${dir}skill.md" | grep "^description:" | sed 's/description: *//')
            echo "  • ${name} — ${desc}"
        fi
    done

# ── Internal ──────────────────────────────────────────────────────────────────

# Ensure skills are downloaded (internal helper)
_ensure-skills:
    #!/usr/bin/env bash
    if [ ! -d "{{skills_dir}}" ] || [ -z "$(ls -A {{skills_dir}} 2>/dev/null)" ]; then
        echo "⚠️  Skills not found, downloading..."
        just download-skills
    fi
