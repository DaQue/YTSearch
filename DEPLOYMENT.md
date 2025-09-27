# Deployment Guide

## 📦 Release Preparation

### Pre-Release Checklist

1. **Code Quality**
   ```bash
   cargo fmt
   cargo clippy --all-targets -- -D warnings
   cargo check --all-targets
   cargo test
   ```

2. **Integration Testing**
   ```bash
   # Test CLI with real API calls
   cargo run --bin probe -- --hours 1 --limit 3
   
   # Test UI functionality
   cargo run
   ```

3. **Documentation**
   - [ ] Update `CHANGELOG.md` with new features/fixes
   - [ ] Verify `README.md` setup instructions
   - [ ] Check `CONTRIBUTING.md` is current
   - [ ] Validate all code comments and docs

4. **Version Management**
   ```bash
   # Update version in Cargo.toml
   sed -i 's/version = "0.1.0"/version = "0.2.0"/' Cargo.toml
   ```

## 🚀 GitHub Deployment

### Initial Repository Setup

1. **Create Repository**
   ```bash
   gh repo create YTSearch --public --description "YouTube search with actual filters - Rust + egui desktop app"
   ```

2. **Setup Repository**
   ```bash
   # Add all files
   git add .
   git commit -m "feat: initial release of YTSearch v0.1.0
   
   - Multi-preset search with date/duration/language filters
   - 3-key API fallback system for quota management  
   - Modern dark UI with channel blocking
   - CLI probe tool for testing"
   
   # Push to GitHub
   git branch -M main
   git remote add origin https://github.com/yourusername/YTSearch.git
   git push -u origin main
   ```

3. **Repository Configuration**
   ```bash
   # Set repository topics
   gh repo edit --add-topic rust,youtube,desktop-app,egui,search-tool,api-client
   
   # Enable issues and discussions
   gh repo edit --enable-issues --enable-discussions
   ```

### Release Creation

```bash
# Create and push tag
git tag -a v0.1.0 -m "Release v0.1.0: Initial working version"
git push origin v0.1.0

# Create GitHub release
gh release create v0.1.0 \
    --title "YTSearch v0.1.0 - Initial Release" \
    --notes-file CHANGELOG.md \
    --draft
```

### Binary Releases (Optional)

```bash
# Build release binaries
cargo build --release

# For Windows cross-compilation (from Linux)
# cargo install cross
# cross build --target x86_64-pc-windows-gnu --release

# Upload binaries to release
gh release upload v0.1.0 target/release/YTSearch
# gh release upload v0.1.0 target/x86_64-pc-windows-gnu/release/YTSearch.exe

# Publish the release
gh release edit v0.1.0 --draft=false
```

## 🔧 Repository Settings

### Branch Protection
```bash
# Protect main branch (if needed later)
gh api repos/:owner/:repo/branches/main/protection \
  --method PUT \
  --field required_status_checks='{"strict":true,"contexts":[]}' \
  --field enforce_admins=true \
  --field required_pull_request_reviews='{"required_approving_review_count":1}' \
  --field restrictions=null
```

### Issue Templates
Create `.github/ISSUE_TEMPLATE/` with:
- `bug_report.md`
- `feature_request.md`  
- `question.md`

### GitHub Actions (Future)
Create `.github/workflows/ci.yml`:
```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    - run: cargo test
    - run: cargo clippy -- -D warnings
```

## 📋 Post-Deployment

1. **Repository Polish**
   - Add screenshots to README
   - Pin important issues
   - Create initial project board
   - Setup GitHub Pages (if needed)

2. **Community**
   - Share in relevant Rust communities
   - Post on social media
   - Add to awesome-rust lists
   - Submit to crates.io (when ready)

3. **Monitoring**
   - Watch for issues and PRs
   - Monitor star count and forks
   - Track user feedback
   - Plan next release cycle

## 🎯 Release Strategy

### Version Naming
- `0.x.y` - Pre-1.0 development
- `1.x.y` - Stable API releases
- Use semantic versioning

### Release Cadence
- **Patch** (0.1.x): Bug fixes, small improvements
- **Minor** (0.x.0): New features, API additions
- **Major** (x.0.0): Breaking changes, major rewrites

### Communication
- Announce releases in GitHub Discussions
- Update social media
- Notify key users/contributors
- Consider blog posts for major releases

---

**Ready to deploy!** 🚀 Follow the steps above to get YTSearch on GitHub and share it with Allison and the world.