#!/bin/bash
# release-help.sh - Helper script for creating releases

set -e

echo "🚀 Text-to-Cypher Release Guide"
echo "==============================="
echo ""

echo "📝 Two ways to create releases:"
echo ""

echo "1️⃣  Tag-based releases (Recommended):"
echo "   # Create and push a version tag"
echo "   git tag v1.0.0"
echo "   git push origin v1.0.0"
echo "   # → This automatically triggers the GitHub Actions workflow"
echo ""

echo "2️⃣  Manual releases via GitHub Actions:"
echo "   # Go to: https://github.com/barakb/text-to-cypher/actions/workflows/build.yml"
echo "   # Click 'Run workflow'"
echo "   # Enter version (e.g., v1.0.0) and optional release name"
echo ""

echo "📦 What gets created:"
echo "   • Cross-compiled binaries for Linux (x86_64, x86_64-musl, aarch64)"
echo "   • Packaged tar.gz files with binaries + templates"
echo "   • Checksums file"
echo "   • Installation script"
echo "   • Docker-ready release structure"
echo ""

echo "🐳 Using releases in Docker:"
echo "   # Download packaged release (includes templates)"
echo "   wget https://github.com/barakb/text-to-cypher/releases/download/v1.0.0/packages/text-to-cypher-linux-x86_64-musl.tar.gz"
echo "   tar -xzf text-to-cypher-linux-x86_64-musl.tar.gz"
echo "   # Now you have: text-to-cypher binary + templates/ directory"
echo ""

echo "🏷️  Version tag format: v<major>.<minor>.<patch> (e.g., v1.0.0, v2.1.3)"
echo "⚡ Only tags starting with 'v' trigger automatic releases"
echo ""

echo "📋 Current available releases:"
echo "   Check: https://github.com/barakb/text-to-cypher/releases"
