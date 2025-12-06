# Release Process

This document describes the release workflow and procedures for fusabi-plugin-runtime.

## Overview

Releases are automated via GitHub Actions and use semantic versioning with conventional commits for changelog generation.

## Release Types

- **Patch** (0.1.x): Bug fixes, documentation updates, non-breaking changes
- **Minor** (0.x.0): New features, non-breaking enhancements
- **Major** (x.0.0): Breaking changes, major redesigns

## Prerequisites

Before releasing:
1. All CI checks must pass
2. Code review and approval from CODEOWNERS
3. Branch protection rules satisfied
4. Documentation updated for new features
5. CHANGELOG.md reviewed and updated

## Release Workflow

### Automated Release (Recommended)

The release workflow is triggered automatically on merge to main when:
- Version in `Cargo.toml` is updated
- Commit messages follow conventional commit format

```bash
# The workflow will:
# 1. Run full CI suite
# 2. Generate changelog from commits
# 3. Create GitHub release
# 4. Build and publish to crates.io
# 5. Archive documentation for this version
```

### Manual Release Steps

If needed, follow these steps for a manual release:

#### 1. Prepare Release Branch

```bash
# Create release branch
git checkout -b release/v0.2.0 main

# Update version in Cargo.toml
# Bump version: 0.1.0 -> 0.2.0
vim Cargo.toml

# Update Cargo.lock
cargo update -p fusabi-plugin-runtime
```

#### 2. Update Documentation

```bash
# Archive current vNEXT docs to versioned folder
cp -r docs/versions/vNEXT docs/versions/v0.2.0

# Update version references in archived docs
find docs/versions/v0.2.0 -type f -name "*.md" -exec sed -i 's/vNEXT/v0.2.0/g' {} +

# Update CHANGELOG.md
vim CHANGELOG.md
```

#### 3. Create Release Commit

```bash
# Commit changes
git add Cargo.toml Cargo.lock docs/ CHANGELOG.md
git commit -m "chore(release): prepare v0.2.0 release

- Update version to 0.2.0
- Archive documentation
- Update changelog

Refs: #123"

# Push release branch
git push origin release/v0.2.0
```

#### 4. Create Pull Request

```bash
# Create PR
gh pr create \
  --title "Release v0.2.0" \
  --body "$(cat <<EOF
## Release v0.2.0

### Changes
- Feature A (#123)
- Feature B (#124)
- Bug fix C (#125)

### Checklist
- [x] Version bumped in Cargo.toml
- [x] CHANGELOG.md updated
- [x] Documentation archived
- [x] CI passing
- [x] Ready for crates.io

Closes #123, #124, #125
EOF
)"
```

#### 5. Merge and Tag

```bash
# After PR approval and merge
git checkout main
git pull origin main

# Create and push tag
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0
```

#### 6. Publish to crates.io

The release workflow will automatically publish to crates.io. If manual publishing is needed:

```bash
# Verify package
cargo package --list

# Dry run
cargo publish --dry-run

# Publish
cargo publish
```

## Branch Protection Rules

The following rules are enforced on the `main` branch:

1. **Required reviews**: At least 1 approval from CODEOWNERS
2. **Required status checks**:
   - CI / Check
   - CI / Format
   - CI / Clippy
   - CI / Test
   - CI / Documentation
   - CI / Doc Check
3. **No force pushes**
4. **No deletions**
5. **Require branches to be up to date**

## Changelog Generation

Changelogs are generated from commit messages following the [Conventional Commits](https://www.conventionalcommits.org/) specification:

- `feat:` - New features (minor version bump)
- `fix:` - Bug fixes (patch version bump)
- `docs:` - Documentation changes
- `chore:` - Maintenance tasks
- `refactor:` - Code refactoring
- `test:` - Test additions/changes
- `perf:` - Performance improvements
- `BREAKING CHANGE:` - Breaking changes (major version bump)

Example:
```
feat(runtime): add hot reload debounce with exponential backoff

Implements configurable debounce timer and exponential backoff
strategy for hot reload to prevent reload storms.

Closes #42
```

## Version Tagging

Tags follow semantic versioning with a `v` prefix:
- `v0.1.0` - Initial release
- `v0.1.1` - Patch release
- `v0.2.0` - Minor release
- `v1.0.0` - Major release

Each tag triggers the release workflow which:
1. Creates GitHub Release with generated notes
2. Publishes to crates.io
3. Updates documentation links

## Publishing to crates.io

### Prerequisites

- crates.io account with publish permissions
- API token configured in GitHub secrets (`CARGO_REGISTRY_TOKEN`)

### Verification Checklist

Before publishing:
- [ ] README.md is up to date
- [ ] Documentation links are valid
- [ ] License files are present
- [ ] Examples run correctly
- [ ] Tests pass on all platforms
- [ ] No dev dependencies leak into published crate

### Post-Publication

After publishing:
1. Verify package on crates.io
2. Check documentation on docs.rs
3. Update repository README with new version
4. Announce release in community channels

## Troubleshooting

### Release Workflow Fails

1. Check GitHub Actions logs for error details
2. Verify all required secrets are configured
3. Ensure crates.io token has publish permissions
4. Check for conflicting version numbers

### Documentation Build Fails

1. Verify all markdown syntax is correct
2. Check for broken internal links
3. Ensure code examples compile
4. Review doc check CI output

### crates.io Publish Fails

1. Verify version number is unique
2. Check package size (must be < 10MB)
3. Ensure all dependencies are published
4. Review crate metadata in Cargo.toml

## Emergency Rollback

If a release has critical issues:

1. **Yank the release from crates.io**:
   ```bash
   cargo yank --vers 0.2.0
   ```

2. **Create hotfix branch**:
   ```bash
   git checkout -b hotfix/v0.2.1 v0.2.0
   # Make fixes
   git commit -m "fix: critical bug in hot reload"
   ```

3. **Follow expedited release process**:
   - Fast-track PR review
   - Publish patch version immediately
   - Update changelog with hotfix notes

## Contact

For release-related questions or issues:
- File an issue: https://github.com/fusabi-lang/fusabi-plugin-runtime/issues
- Contact maintainers via CODEOWNERS
