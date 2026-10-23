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

### Workflow Overview

GitHub Actions pipeline `.github/workflows/rust.yml` automates building, testing, and creating releases with multi-platform binaries.

### Target Platforms Built in CI/CD
- **macOS arm64** (`aarch64-apple-darwin`)
- **macOS x64** (`x86_64-apple-darwin`)
- **Linux x64** (`x86_64-unknown-linux-gnu`)
- **Linux arm64** (`aarch64-unknown-linux-gnu`)

---

## Triggering a Release

Releases are triggered strictly by git tags matching `v*.*.*` (e.g. `v0.1.0`).

### Step-by-Step:
1. Ensure all changes are committed and pushed to `main`.
2. Run local tests:
   ```sh
   /bin/bash compile.sh
   /bin/bash test.sh
   ```
3. Tag the release:
   ```sh
   git tag v0.1.0
   ```
4. Push the tag to GitHub:
   ```sh
   git push origin v0.1.0
   ```

---

## What the CI/CD Pipeline Does

1. **Validation & Test Job**:
   - Runs `compile.sh` and executes `test.sh` on macOS arm64.
2. **Build Matrix Job**:
   - Compiles release binaries for each supported platform.
   - Names binaries accordingly:
     - `securenv-darwin-arm64`
     - `securenv-darwin-x64`
     - `securenv-linux-x64`
     - `securenv-linux-arm64`
3. **GitHub Release**:
   - Creates a new GitHub Release corresponding to the git tag.
   - Attaches all compiled platform binaries as release assets.
