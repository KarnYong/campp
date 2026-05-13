# GitHub Workflow & Release Guide for CAMPP

## Overview

CAMPP uses a single GitHub Actions workflow (`.github/workflows/build.yml`) that builds for macOS, Windows, and Linux, then creates a GitHub release.

## Current Version

Check the version in `package.json`:
```bash
node -p "require('./package.json').version"
```

## Prerequisites

- Push access to the repository
- `gh` CLI installed and authenticated (`gh auth status`)
- Working directory must be clean (`git status`)

---

## Option 1: Tag-Based Release (Recommended)

### Full Release

```bash
# 1. Update version in package.json (if needed)
#    Edit package.json "version" field, e.g. "0.3.0"

# 2. Commit the version bump
git add package.json
git commit -m "chore: bump version to 0.3.0"

# 3. Create an annotated tag
git tag -a v0.3.0 -m "Release v0.3.0"

# 4. Push commit and tag
git push origin main
git push origin v0.3.0
```

### Pre-Release

```bash
# 1. Create a pre-release tag (use -pre, -beta, -rc suffix)
git tag -a v0.3.0-beta.1 -m "Pre-release v0.3.0-beta.1"

# 2. Push the tag
git push origin v0.3.0-beta.1
```

The workflow creates a **draft release** automatically. You must publish it manually.

---

## Option 2: Manual Workflow Dispatch

```bash
# Trigger without creating a tag
gh workflow run build.yml -f create_release=true

# Trigger without creating a release (build only)
gh workflow run build.yml -f create_release=false
```

Note: The release tag name will default to the branch name (e.g., `main`).

---

## After the Workflow Runs

### Monitor the Build

```bash
# List recent runs
gh run list --limit 5

# Watch a specific run in real-time
gh run watch <run-id>

# View run details
gh run view <run-id>

# Check status of all jobs in a run
gh run view <run-id> --json jobs -q '.jobs[] | "\(.name): \(.status) \(.conclusion)"'
```

### Download Artifacts (without releasing)

```bash
# List artifacts from a run
gh run view <run-id> --json artifacts -q '.artifacts[] | .name'

# Download all artifacts
gh run download <run-id>

# Download a specific artifact
gh run download <run-id> -n CAMPP-Windows
gh run download <run-id> -n CAMPP-macOS
gh run download <run-id> -n CAMPP-Linux
```

### Publish the Release

The workflow creates a **draft release**. To publish it:

```bash
# List releases (drafts included)
gh release list --limit 5

# View a specific release
gh release view <tag>

# Publish as a full release
gh release edit <tag> --draft=false

# Publish as a pre-release
gh release edit <tag> --draft=false --prerelease

# Add release notes
gh release edit <tag> --notes "## What's Changed
- Feature X
- Bug fix Y"
```

### Delete a Release (if something went wrong)

```bash
# Delete the release (keeps the tag)
gh release delete <tag>

# Delete the tag too
git push origin --delete <tag>
```

---

## Useful Tag Management

```bash
# List all version tags (sorted newest first)
git tag -l 'v*' --sort=-v:refname

# Delete a local tag
git tag -d <tag>

# Delete a remote tag
git push origin --delete <tag>

# Fetch all remote tags
git fetch --tags
```

---

## Complete Pre-Release Example (Start to Finish)

```bash
# 1. Check current state
git status
node -p "require('./package.json').version"

# 2. Create and push pre-release tag
git tag -a v0.3.0-rc.1 -m "Release candidate v0.3.0-rc.1"
git push origin v0.3.0-rc.1

# 3. Get the run ID
gh run list --limit 1 --json databaseId -q '.[0].databaseId'

# 4. Watch the build (optional)
gh run watch <run-id>

# 5. Once complete, publish as pre-release
gh release edit v0.3.0-rc.1 --draft=false --prerelease
```

## Complete Full Release Example (Start to Finish)

```bash
# 1. Bump version
# Edit package.json → "version": "0.3.0"

# 2. Commit and tag
git add package.json
git commit -m "chore: bump version to 0.3.0"
git tag -a v0.3.0 -m "Release v0.3.0"

# 3. Push
git push origin main v0.3.0

# 4. Monitor
gh run list --limit 1

# 5. Publish
gh release edit v0.3.0 --draft=false
```

---

## Build Artifacts by Platform

| Platform   | Artifacts                                    |
|------------|----------------------------------------------|
| macOS      | `CAMPP-<version>-arm64.dmg`                  |
|            | `CAMPP-<version>-x64.dmg`                    |
|            | `CAMPP-<version>-universal.dmg`              |
| Windows    | `CAMPP-<version>-x64.msi`                    |
|            | `CAMPP-<version>-x64.exe` (NSIS installer)   |
| Linux      | `CAMPP-<version>-amd64.deb`                  |
|            | `CAMPP-<version>-amd64.AppImage`             |

## Troubleshooting

**Workflow didn't trigger**: Ensure the tag starts with `v` (e.g., `v0.3.0`, not `0.3.0`).

**Build failed**: Check logs with `gh run view <run-id> --log-failed`.

**Release not visible**: Draft releases are only visible to repo maintainers. Publish with `gh release edit <tag> --draft=false`.

**Wrong tag**: Delete and re-create:
```bash
git push origin --delete <wrong-tag>
git tag -d <wrong-tag>
git tag -a <correct-tag> -m "Message"
git push origin <correct-tag>
```
