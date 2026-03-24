# Contributing

Thanks for helping improve Rustatio.

This guide explains how to set up the project, run it locally, and submit changes with the same workflow used in the repo.

## Project Layout

Rustatio is a multi-crate workspace with a shared UI:

- `rustatio-desktop/` - Tauri desktop app
- `rustatio-core/` - shared core logic
- `rustatio-server/` - self-hosted server
- `rustatio-cli/` - CLI app
- `rustatio-wasm/` - WASM bindings
- `ui/` - shared web UI for desktop/server/web

## Prerequisites

The easiest path is to use `mise`, which installs the Rust and Node tooling defined by the project.

If you do not want to install `mise` globally first, this repo includes a bootstrapped wrapper at `scripts/mise` that will download and run the pinned `mise` version for you.

Install `mise` first:

- https://mise.jdx.dev/

Or use the bundled bootstrap script directly:

```bash
./scripts/mise run install
```

Then install project tools and dependencies:

```bash
mise run install
```

If you use the bootstrap script, you can replace `mise ...` in the commands below with `./scripts/mise ...`.

That will install:

- Rust + `rustfmt` + `clippy`
- Node.js
- `hk` and repo lint tools
- UI dependencies

## Quick Start

Pick the version you want to run.

### Run the server

```bash
mise run run:server
```

Then open:

- `http://localhost:8080`

### Run the desktop app

```bash
mise run run:desktop
```

### Run the web version

```bash
mise run run:web
```

This starts the Vite dev server after building the WASM bindings.

### Run the CLI

```bash
mise run run:cli -- --help
```

## Useful Commands

### Build

```bash
mise run build:server
mise run build:desktop
mise run build:web
mise run build:cli
```

### Test

Run the full test suite:

```bash
mise run test
```

Run UI tests only:

```bash
mise run ui:test
```

### Lint and checks

Run all repo checks:

```bash
hk check --all
```

Auto-fix what can be fixed automatically:

```bash
hk fix --all
```

Run the full CI workflow locally:

```bash
mise ci
```

## Docker Development

Build and run the Docker image locally:

```bash
mise run docker:build
mise run docker:run
```

Stop it again:

```bash
mise run docker:stop
```

## Contribution Workflow

1. Create a branch for your change.
2. Make the smallest focused change that solves the issue.
3. Add tests for new features and regressions.
4. Run checks before opening a PR.
5. Open a pull request with a clear description of the change and why it is needed.

Recommended pre-PR commands:

```bash
mise ci
```

## Project Rules

Please follow these repo conventions:

- Use `mise` tasks when available.
- Run unit tests after code changes.
- Add tests for new features. This is mandatory.
- In Rust, avoid `unwrap`, `expect`, and similar panic-inducing APIs.
- Prefer clear error handling over silent failure.
- In JavaScript, handle errors explicitly and consistently.

## Pull Requests

Good pull requests usually include:

- a short explanation of the problem
- what changed
- how it was tested
- screenshots or recordings for UI changes

Small, focused PRs are much easier to review than large mixed changes.

## Need Help?

If you are not sure where to start:

- open an issue
- ask a clarifying question in the PR
- propose the smallest possible first step

Thanks for contributing.
