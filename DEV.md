# Developer Guide

## Local Toolchain Setup

### Install Rust

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Add the macOS arm64 target

```sh
rustup target add aarch64-apple-darwin
```

### Verify toolchain

Run the validation script to confirm everything is in order:

```sh
/bin/bash validateenv.sh
```

Exit 0 means the local toolchain is ready for development.

## Build locally

```sh
/bin/bash compile.sh
```

## Running Tests Locally

```sh
/bin/bash test.sh
```

---

## Release and Publishing Process

### CI/CD Workflow Overview

GitHub Actions pipeline `.github/workflows/rust.yml` handles automated building, testing, and release creation:
- **Pushing code to any branch** (or opening a PR) triggers the CI test suite (`compile.sh` and `test.sh`) across macOS arm64 and builds cross-platform binaries to ensure build stability. It does **not** create any release.
- **Pushing a git tag** triggers the CI test suite, compiles all binaries, and generates a new **GitHub Draft Release**.

### Target Platforms Built in CI/CD
- **macOS arm64** (`aarch64-apple-darwin` → `securenv-darwin-arm64`)
- **Linux x64** (`x86_64-unknown-linux-gnu` → `securenv-linux-x64`)
- **Linux arm64** (`aarch64-unknown-linux-gnu` → `securenv-linux-arm64`)

---

## Creating a Release

Use the helper script `tag.sh` to bump version and push tag:

```sh
/bin/bash tag.sh patch   # or minor, major
```

Example session:
```text
Fetching git tags...

Previous tag: v0.0.4
Mode:         patch
New tag:      v0.0.5

Press Enter to create and push tag 'v0.0.5', or Ctrl+C to abort...
Enumerating objects: 9, done.
...
To github.com:stopsopa/securenv.git
 * [new tag]         v0.0.5 -> v0.0.5
Successfully created and pushed tag >v0.0.5<
```

### What happens next

1. GitHub Actions detects the tag push and runs the pipeline.
2. It compiles binaries for macOS arm64, Linux x64, and Linux arm64.
3. Computes SHA-256 hashes for all binaries and generates dynamic `curl`, `wget`, and verification commands.
4. Creates and publishes the GitHub Release with attached platform binaries and generated checksum verification notes.
