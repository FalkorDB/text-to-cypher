# Version Control and Release Management

This document explains how to control versions and releases for the text-to-cypher project.

## 🎯 Release Methods

### **Method 1: Manual Release (Recommended)**

Use the GitHub Actions UI to create releases with custom versions:

1. **Go to GitHub Actions** → Select "build" workflow
2. **Click "Run workflow"**
3. **Fill in the parameters:**
   - **Version**: `v1.0.0` (semantic versioning)
   - **Release name**: `Major Release v1.0.0` (optional, defaults to "Release v1.0.0")
   - **Prerelease**: Check if this is a beta/alpha version

### **Method 2: Git Tag Release (Automatic)**

Push a git tag to automatically create a release:

```bash
# Create and push a version tag
git tag v1.0.0
git push origin v1.0.0

# For prereleases (will be marked as prerelease automatically)
git tag v1.0.0-alpha.1
git push origin v1.0.0-alpha.1
```

### **Method 3: Auto Release on Push**

Every push to `master` creates an automatic release with date-based versioning:

- Format: `v2025.07.23-abc1234`
- Always marked as stable release

## 📋 Version Naming Conventions

### **Semantic Versioning (Recommended)**

```
v1.0.0          # Major release
v1.1.0          # Minor update
v1.1.1          # Patch/bugfix
v2.0.0-alpha.1  # Pre-release alpha
v2.0.0-beta.2   # Pre-release beta
v2.0.0-rc.1     # Release candidate
```

### **Date-based Versioning**

```
v2025.07.23-abc1234  # Auto-generated from date + commit
v2025.07.23          # Manual date-based
```

### **Custom Naming**

```
v1.0.0-hotfix       # Hotfix release
v1.0.0-experimental # Experimental features
v1.0.0-stable       # Stable release
```

## 🛠️ How to Use Each Method

### **1. Manual Release via GitHub UI**

#### Step-by-step

1. Navigate to your repository on GitHub
2. Click **Actions** tab
3. Select **build** workflow from the left sidebar
4. Click **Run workflow** button (top right)
5. Fill in the form:

   ```
   Version: v1.0.0
   Release name: Major Release - New MCP Features
   Prerelease: [ ] (unchecked for stable)
   ```

6. Click **Run workflow**

#### Benefits

- ✅ Full control over version and name
- ✅ Can mark as prerelease
- ✅ Can be triggered anytime
- ✅ Custom release descriptions

### **2. Git Tag Method**

#### For stable releases

```bash
# Ensure you're on the latest master
git checkout master
git pull origin master

# Create and push tag
git tag v1.0.0
git push origin v1.0.0
```

#### For prereleases

```bash
# Alpha release
git tag v1.0.0-alpha.1
git push origin v1.0.0-alpha.1

# Beta release  
git tag v1.0.0-beta.1
git push origin v1.0.0-beta.1

# Release candidate
git tag v1.0.0-rc.1
git push origin v1.0.0-rc.1
```

#### Benefits

- ✅ Follows Git best practices
- ✅ Automatic prerelease detection
- ✅ Version control integration
- ✅ Can be scripted/automated

### **3. Automatic on Push**

#### How it works

- Every push to `master` automatically creates a release
- Version format: `v{YYYY.MM.DD}-{short-commit-hash}`
- Example: `v2025.07.23-a1b2c3d`

#### Benefits

- ✅ Zero configuration
- ✅ Always creates releases
- ✅ Good for continuous delivery

## 🔧 Advanced Configuration

### **Customize the VERSION file**

Update the `VERSION` file in your repository:

```bash
echo "1.0.0" > VERSION
git add VERSION
git commit -m "Bump version to 1.0.0"
```

### **Script-based Releases**

Create a release script:

```bash
#!/bin/bash
# release.sh
set -e

VERSION=${1:-$(cat VERSION)}
RELEASE_NAME=${2:-"Release $VERSION"}

echo "Creating release $VERSION..."

# Create and push tag
git tag "v$VERSION"
git push origin "v$VERSION"

echo "Release v$VERSION created successfully!"
echo "Visit: https://github.com/FalkorDB/text-to-cypher/releases"
```

Usage:

```bash
./release.sh 1.0.0 "Major Release - MCP Support"
```

### **Automated Version Bumping**

Add version bumping to your workflow:

```yaml
- name: Bump version
  run: |
    # Read current version
    CURRENT=$(cat VERSION)
    
    # Increment patch version (basic example)
    NEW_VERSION=$(echo $CURRENT | awk -F. '{$NF = $NF + 1;} 1' | sed 's/ /./g')
    
    # Update VERSION file
    echo $NEW_VERSION > VERSION
    
    # Commit if needed
    git config user.name "GitHub Actions"
    git config user.email "actions@github.com"
    git add VERSION
    git commit -m "Bump version to $NEW_VERSION" || exit 0
```

## 📊 Comparison Table

| Method | Control | Ease | Automation | Best For |
|--------|---------|------|------------|----------|
| Manual UI | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ | Major releases, custom names |
| Git Tags | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | Standard releases, CI/CD |
| Auto Push | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | Development, continuous delivery |

## 🎯 Recommended Workflow

### **For Development:**

1. Use **auto-push releases** for daily builds
2. Version format: `v2025.07.23-abc1234`

### **For Stable Releases:**

1. Use **git tags** for version control integration
2. Follow semantic versioning: `v1.0.0`, `v1.1.0`, etc.

### **For Special Releases:**

1. Use **manual UI** for custom names and descriptions
2. Examples: "Hotfix Release", "Beta with New Features"

## 🔄 Migration Strategy

### **From Auto to Manual:**

1. Disable auto releases by modifying the workflow condition
2. Switch to manual releases for better control

### **Current Setup:**

Your repository supports all three methods simultaneously:

- ✅ Auto releases on push to master
- ✅ Manual releases via GitHub UI  
- ✅ Tag-based releases via git push

Choose the method that best fits your release strategy!

## 🧹 Release Cleanup & Cost Management

### **Automatic Cleanup**

To manage GitHub storage costs, the repository is configured to automatically keep only the **last 10 releases**:

- **Cleanup runs automatically** after each new release
- **Older releases are deleted** including their binaries and tags
- **Weekly scheduled cleanup** runs every Sunday at 2 AM UTC
- **No impact on git history** - only GitHub releases are affected

### **Manual Cleanup**

You can also run cleanup manually with custom settings:

1. **Go to GitHub Actions** → Select "Cleanup Old Releases" workflow
2. **Click "Run workflow"**
3. **Specify number of releases to keep** (default: 10)

### **Cleanup Configuration**

The cleanup behavior can be modified in:

- `.github/workflows/cleanup-releases.yml` - Standalone cleanup workflow
- `.github/workflows/build.yml` - Cleanup after manual releases
- `.github/workflows/release.yml` - Cleanup after tag-based releases

### **Cost Benefits**

- **Reduces storage costs** by removing old release binaries
- **Maintains recent releases** for active development
- **Keeps repository lightweight** and focused on current versions
- **Automatic management** requires no manual intervention

### **What Gets Cleaned Up**

- ✅ Release entries and their binary attachments
- ✅ Associated git tags
- ✅ Release notes and metadata
- ❌ Git commit history (preserved)
- ❌ Source code (preserved)
