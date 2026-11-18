# Contributing to QuestForge

Thank you for contributing to **QuestForge**!

## Getting Started

1. Fork the repository and create a feature branch from `main`.
2. See [docs/4-development/developer_guide.md](docs/4-development/developer_guide.md) for workspace setup and coding conventions.
3. Ensure all checks pass locally before opening a pull request:
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
   - `swartch --vocabulary config/vocabulary.toml ...` (0 offences)

## Pull Requests

- Keep pull requests focused and include tests for new functionality.
- Reference related issues (e.g. `Closes #123`).
- Use the PR template in `.github/PULL_REQUEST_TEMPLATE.md`.
