# HTMX Rust Todo App

A full-stack Rust web application implementing todo/task management, recipe management, and meal planning with HTMX for dynamic interactions.

## Features

- **Todo Lists & Tasks**: Create, organize, and manage todo lists with drag-and-drop reordering
- **Recipe Management**: Create recipes with photos, ingredients, and instructions
- **Meal Planning**: Weekly meal planning with recipe integration
- **Photo Upload**: Multi-photo upload with automatic thumbnail generation

## Tech Stack

- **Backend**: Rust + Axum + SQLite + Askama templates
- **Frontend**: HTMX + PicoCSS + minimal JavaScript (drag-and-drop)
- **Testing**: Rust integration tests + Playwright E2E tests

## Usage

### NixOS (Recommended)

#### Run Application

```bash
# Run with defaults (port 3000, localhost)
nix run

# Run with custom options
nix run -- --port 8080 --address 0.0.0.0 --data-dir ./data
```

#### Development Environment

```bash
# Enter development shell with all dependencies
nix develop

# Run tests (requires nix develop environment)
nix develop --command cargo test

# Run E2E tests
npm test
```

### Using Cargo

```bash
# Build and run
cargo run -- --port 3000 --address 127.0.0.1

# Run tests (may require additional system dependencies)
cargo test
```

## Test Status

✅ **100% Pass Rate** - All 40 core integration tests passing:
- Integration Tests: 11/11
- Meal Plan Tests: 13/13  
- Recipe Tests: 14/14
- Photo Upload Tests: 2/2

## Documentation

See [CLAUDE.md](./CLAUDE.md) for comprehensive architecture, development, and testing documentation.
