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

Produces `bin/arm64` (macOS arm64 binary).
