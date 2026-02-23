# Contributing to Gumol Viz Engine

Thank you for your interest in contributing to Gumol Viz Engine! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Submitting Changes](#submitting-changes)

## Code of Conduct

This project adheres to a code of conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to [your.email@example.com](mailto:your.email@example.com).

## Getting Started

### Prerequisites

- Rust 1.75 or higher
- Git
- A code editor (VS Code with rust-analyzer is recommended)

### Setup

```bash
# Fork and clone the repository
git clone https://github.com/yourusername/gumol-viz-engine.git
cd gumol-viz-engine

# Install development tools
cargo install cargo-watch
cargo install cargo-edit

# Install useful VS Code extensions
code --install-extension rust-lang.rust-analyzer
code --install-extension tamasfe.even-better-toml
code --install-extension usernamehw.errorlens
```

### Build and Test

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run linter
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## Development Workflow

### Branching

1. Create a branch from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes

3. Commit your changes (use conventional commits):
   ```bash
   git commit -m "feat: add XYZ file parser"
   ```

4. Push to your fork:
   ```bash
   git push origin feature/your-feature-name
   ```

5. Create a Pull Request

### Conventional Commits

We use conventional commit messages:

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation only changes
- `style:` - Code style changes (formatting)
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `test:` - Adding or updating tests
- `chore:` - Maintenance tasks

Example:
```
feat: add PDB CONECT record parsing

This adds support for parsing CONECT records in PDB files,
which explicitly define bonds between atoms.

Closes #123
```

## Coding Standards

### Rust Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Pass `cargo clippy -- -D warnings`
- Prefer `&str` over `String` for function parameters when possible
- Use `Result<T, E>` for error handling, not panics
- Document all public APIs with doc comments

### Naming Conventions

- Types: `PascalCase`
- Functions: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Acronyms: `PdbParser`, `XyzWriter` (not `PDBParser`, `XYZWriter`)

### Example Code

```rust
use bevy::prelude::*;

/// Parse a PDB file and return trajectory data
///
/// # Arguments
///
/// * `path` - Path to the PDB file
///
/// # Returns
///
/// Returns a `Result` containing the `Trajectory` or an error
///
/// # Errors
///
/// Returns an error if the file cannot be read or is invalid
///
/// # Examples
///
/// ```
/// let trajectory = PDBParser::parse_file("protein.pdb")?;
/// ```
pub fn parse_file(path: &Path) -> IOResult<Trajectory> {
    // Implementation
    unimplemented!()
}
```

## Testing

### Unit Tests

Write unit tests in the same module as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pdb() {
        let content = "ATOM      1  N   ALA A   1       0.0   0.0   0.0";
        let result = PDBParser::parse_string(content);
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Place integration tests in the `tests/` directory:

```rust
// tests/integration_test.rs
use gumol_viz_engine::GumolVizPlugin;

#[test]
fn test_plugin_registration() {
    // Test the plugin works correctly
}
```

### Run Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_parse_simple_pdb
```

## Documentation

### Code Documentation

Document all public APIs:

```rust
/// Represents a molecular trajectory
///
/// Contains atom positions and metadata for multiple frames.
pub struct Trajectory {
    /// Path to the trajectory file
    pub file_path: PathBuf,

    /// All frames in the trajectory
    pub frames: Vec<FrameData>,
}
```

### Examples

Provide examples in documentation:

```rust
/// Parses an XYZ file
///
/// # Examples
///
/// ```
/// use gumol_viz_engine::io::xyz::XYZParser;
///
/// let trajectory = XYZParser::parse_file("water.xyz")?;
/// println!("Loaded {} frames", trajectory.num_frames());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_file(path: &Path) -> IOResult<Trajectory> {
    // ...
}
```

### README Updates

When adding features:
1. Update the feature list in README.md
2. Add usage examples
3. Update the roadmap if applicable

## Submitting Changes

### Before Submitting

1. ✅ Run `cargo fmt`
2. ✅ Run `cargo clippy -- -D warnings`
3. ✅ Run `cargo test`
4. ✅ Update documentation
5. ✅ Add/update tests

### Pull Request

1. Create a PR from your feature branch to `main`
2. Fill in the PR template
3. Link related issues
4. Request review from maintainers

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Added unit tests
- [ ] Added integration tests
- [ ] All tests pass

## Checklist
- [ ] Code follows style guidelines
- [ ] Documentation updated
- [ ] Commit messages follow convention
- [ ] No merge conflicts
```

## Getting Help

- Read the [Architecture Guide](docs/ARCHITECTURE.md)
- Check [existing issues](https://github.com/yourusername/gumol-viz-engine/issues)
- Start a [discussion](https://github.com/yourusername/gumol-viz-engine/discussions)

## Recognition

Contributors will be acknowledged in the CONTRIBUTORS.md file.

---

Thank you for contributing to Gumol Viz Engine! 🚀
