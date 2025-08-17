# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Purpose

This is personal software used to implement some conveniences for a family. The website is never expected to 
receive much traffic, so the software herein should focus on simplicity, ease of understanding, and ease of 
maintenance rather than performance. Each page in the website should be reasonably independent. 
The most important feature is the todo app.

## Project Overview

This is a full-stack Rust web application implementing a Todo/Task management system with Recipe management and Meal planning features, plus optional Grocy integration. It uses HTMX for dynamic frontend interactions without heavy JavaScript frameworks.

**Tech Stack**: Rust + Axum + SQLite + HTMX + PicoCSS + Askama templates + Chrono

## Development Commands

### Using Nix (Recommended)
```bash
# Enter development environment
nix develop

# Run application (with default settings)
nix run

# Run with custom options
nix run -- --port 8080 --address 0.0.0.0 --data-dir ./data
```

### Using Cargo
```bash
# Build project
cargo build

# Run in development
cargo run

# Build for release
cargo build --release

# Run with options
cargo run -- --port 8080 --address 0.0.0.0 --data-dir ./data
```

## Architecture

**Backend Structure:**
- `src/main.rs` - Web server entry point
- `src/lib.rs` - Main application logic and route handlers (Axum)
- `src/database.rs` - SQLite database operations and migrations
- `src/todo.rs` - Core data structures (Task, List, Recipe, MealPlanEntry, forms)
- `src/grocy.rs` - External Grocy API integration
- `src/template.rs` - Askama template definitions

**Frontend Pattern:**
- Server-side rendered HTML templates in `templates/`
- HTMX handles dynamic updates without page reloads
- Minimal JavaScript only for drag-and-drop (Sortable.js)
- Vendored assets in `vendor/` directory (htmx.js, Sortable.js, pico.css)

**Database:**
- SQLite schema in `sql/schema.sql`
- Tables: `lists`, `tasks`, `grocy_credentials`, `grocy_tasks_mapping`, `recipes`, `meal_plan`
- Auto-migration handled in database.rs

## Key Patterns

**HTMX Integration:**
- Routes return HTML fragments for dynamic updates
- Forms use HTMX attributes for seamless submission
- Drag-and-drop reordering via POST to `/reorder` endpoint

**Template System:**
- Askama templates provide type-safe server-side rendering
- Templates located in `templates/` directory
- Each template corresponds to specific UI components

**Error Handling:**
- Uses `anyhow` for error context throughout codebase
- Database errors bubble up through Result types
- HTTP errors return appropriate status codes

## Important Notes

- Frontend assets are vendored and gzipped in `vendor/` directory
- Comprehensive testing with integration tests (axum_test) and E2E tests (Playwright)
- Application uses async/await throughout with Tokio runtime
- Grocy integration is optional and configurable per todo list
- Task ordering maintained via integer `position` field with reordering logic
- Recipe and meal planning features are free-form and mobile-first
- Recipes support title, ingredients (one per line), instructions with auto-linked URLs, and optional photos

## New Features

### Recipe Management (`/recipes`)
- Create, read, update, delete recipes
- Free-form ingredients list (no units or structured data)
- Instructions with automatic URL link detection
- Photo upload with auto-submit functionality
- Recipe-to-todo-list integration with ingredient selection
- Simple search by title (future enhancement)

### Meal Planning (`/meal-plan`)
- Weekly view of planned meals
- Add recipes or free-form meal descriptions to any day
- Navigate between weeks
- Integration with recipe collection
- Recipe-to-meal-plan integration via "Add to Meal Plan" buttons
- Mobile-first responsive design
- Simplified workflow: meal plan → "Browse Recipes" → select recipe → "Add to Meal Plan"

### Navigation
- Recipe and Meal Plan links added to main navigation
- Consistent header across all pages
- Mobile-optimized layouts

## Testing

The application has comprehensive test coverage using two complementary approaches:

### Integration Tests (Rust)
Integration tests using `axum_test` framework test the backend functionality:

```bash
# Run all integration tests
cargo test

# Run specific test modules
cargo test recipe_tests
cargo test meal_plan_tests
cargo test todo_tests
```

**Test Coverage:**
- Recipe CRUD operations and photo uploads (`tests/recipe_tests.rs`)
- Meal plan functionality and week navigation (`tests/meal_plan_tests.rs`) 
- Todo list management and task operations (`tests/todo_tests.rs`)
- Database operations and schema validation
- Form validation and error handling

### End-to-End Tests (Playwright)
E2E tests using Playwright provide "outside perspective" testing of the full user experience:

```bash
# Enter nix development environment (includes Playwright)
nix develop

# Run all E2E tests
npm test

# Run tests in headed mode (with browser UI)
npm run test:headed

# Run tests in debug mode
npm run test:debug

# View test results
npm run test:report

# Run tests with UI mode
npm run test:ui
```

**E2E Test Coverage:**
- **Basic App Functionality** (`e2e-tests/01-basic.spec.ts`)
  - Homepage loading and navigation
  - Todo list creation and management
  - Task creation, completion, and toggling
  - Mobile responsiveness
  - Vendor asset loading (HTMX, PicoCSS)

- **Recipe Management** (`e2e-tests/02-recipes.spec.ts`)
  - Recipe creation, editing, and deletion
  - Photo upload functionality with auto-submit
  - Recipe-to-todo-list integration
  - URL auto-linking in instructions
  - Mobile recipe views

- **Meal Planning** (`e2e-tests/03-meal-plan.spec.ts`)
  - Week navigation and layout
  - Adding free-form meals and recipes to days
  - Meal entry editing and deletion
  - Recipe integration in meal planning
  - Mobile meal plan interface

- **Drag & Drop** (`e2e-tests/04-drag-drop.spec.ts`)
  - Task reordering via drag and drop
  - Persistence across page reloads
  - Mixed completed/incomplete task handling
  - Mobile drag behavior
  - Visual feedback and error handling

**E2E Test Configuration:**
- Multi-browser testing (Chrome, Firefox, Safari, Mobile Chrome, Mobile Safari)
- Automatic test server startup (port 3001)
- Screenshots and videos on failure
- Isolated test data directory (`./e2e-test-data`)

### Manual Testing with curl
```bash
# Test recipes page
curl http://localhost:3001/recipes

# Test meal plan page  
curl http://localhost:3001/meal-plan

# Test recipe creation form
curl http://localhost:3001/recipes/new

# Create a recipe (example)
curl -X POST http://localhost:3001/recipes/new \
  -d "title=Test Recipe" \
  -d "ingredients=1 cup flour%0A2 eggs" \
  -d "instructions=Mix ingredients and bake"

# Test meal plan form
curl http://localhost:3001/meal-plan/2025-08-17/add
```

### Database Schema Testing
```bash
# Connect to SQLite database to verify schema
sqlite3 todos.db ".schema"
sqlite3 todos.db "SELECT * FROM recipes;"
sqlite3 todos.db "SELECT * FROM meal_plan;"
```
