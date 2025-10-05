# Contributing to YTSearch

Thanks for your interest in contributing to YTSearch! This document provides guidelines for contributing to this YouTube search tool.

## 🚀 Quick Start

1. **Fork & Clone**
   ```bash
   git clone https://github.com/yourusername/YTSearch.git
   cd YTSearch
   ```

2. **Setup API Keys**
   ```bash
   # Place your YouTube Data API v3 key in any of these files:
   echo "YOUR_API_KEY_HERE" > YT_API_private
   echo "YOUR_BACKUP_KEY" > YT_API_private.alt
   echo "YOUR_THIRD_KEY" > YT_API_private,old
   ```

3. **Dry Run CLI**
   ```bash
   cargo run --bin probe -- --dry-run
   ```

## 🏗️ Project Structure

```
src/
├── main.rs              # Entry point
├── lib.rs               # Module exports
├── ui/                  # User interface
│   ├── mod.rs           # eframe::App implementation
│   ├── app_state.rs     # Application state & logic
│   ├── panels.rs        # UI panel rendering
│   ├── theme.rs         # Colors & styling
│   └── utils.rs         # Utility functions
├── search_runner.rs     # Search orchestration
├── prefs.rs            # Preferences & persistence
├── filters.rs          # Post-search filtering
├── yt/                 # YouTube API client
│   ├── mod.rs
│   ├── search.rs       # search.list endpoint
│   ├── videos.rs       # videos.list endpoint
│   └── types.rs        # API response types
└── bin/
    └── probe.rs        # CLI testing tool
```

## 🎯 Areas for Contribution

### High Priority
- **Preset Editor UI**: Replace placeholder with full CRUD operations
- **OAuth Integration**: Add `captions.list` support for precise English filtering
- **Export Features**: HTML reports, CSV export
- **Performance**: Caching, request batching optimizations

### Medium Priority
- **Enhanced Filtering**: Description text search, category filtering
- **UI Improvements**: Keyboard shortcuts, thumbnail display, progress indicators
- **Error Recovery**: Network retry with exponential backoff
- **Testing**: Unit tests for filters, integration tests for API calls

### Nice to Have
- **Playlist Support**: Channel uploads, saved playlists
- **Auto-refresh**: Background search scheduling
- **Custom Themes**: User-configurable color schemes
- **Localization**: Multi-language support

## 🛠️ Development Guidelines

### Code Style
- Stick with Rust 2021 edition defaults; keep formatting and lint warnings clean before sending patches.
- **Error handling**: Use `anyhow` for error propagation, meaningful error messages
- **Async**: Tokio runtime, non-blocking UI operations
- **Documentation**: Public APIs documented with `///` comments

### Testing
- When you need to exercise the live API, run:
  ```bash
  cargo run --bin probe -- --hours 1 --limit 3
  ```
- Launch the UI after changes to verify visuals and shortcuts:
  ```bash
  cargo run
  ```

### Commit Messages
- Keep commits focused and consider conventional prefixes like `feat:`, `fix:`, or `docs:` when they help reviewers.

## 🔑 API Key Management

**Important**: Never commit API keys to version control!

- Use `.gitignore` patterns for `YT_API_private*` files
- Store keys locally in the documented file locations
- Test with quota-limited keys to verify fallback behavior

## 📝 Pull Request Process

1. **Create Feature Branch**
   ```bash
   git checkout -b feature/preset-editor-ui
   ```

2. **Make Changes**
   - Follow the project structure and coding conventions
   - Add tests for new functionality
   - Update documentation as needed

3. **Test Thoroughly**
   - Make sure automated checks pass locally.
   - See the Testing section below for project-specific checks.

4. **Submit PR**
   - Clear description of changes and motivation
   - Reference any related issues
   - Include screenshots for UI changes

## 🐛 Bug Reports

**Great bug reports include:**
- Clear description of expected vs actual behavior
- Minimal reproduction steps
- Environment details (OS, Rust version)
- Error messages and logs
- API key status (working/quota exceeded/restricted)

**Template:**
```markdown
## Bug Description
Brief summary of the issue

## Steps to Reproduce
1. Configure API key in YT_API_private
2. Run `cargo run --bin probe -- --hours 24`
3. Observe error: "..."

## Expected Behavior
Should return search results without errors

## Environment
- OS: Ubuntu 22.04
- Rust: 1.70.0
- YTSearch: main branch @ abc1234

## Additional Context
Any relevant details about API quotas, network setup, etc.
```

## 💡 Feature Requests

We love feature ideas! Please check existing issues first, then create a new issue with:
- **Use case**: What problem does this solve?
- **Proposed solution**: How should it work?
- **Alternatives**: What other approaches did you consider?
- **Implementation notes**: Any technical considerations

## 📄 License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

**Questions?** Open an issue or reach out to the maintainers. Thanks for helping make YouTube search better! 🎉