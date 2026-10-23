# Release Notes

### CI/CD Optimization: Switch Pipeline Test Environment to Linux x64

- **CI Test Suite**: Migrated test job runner from macOS arm64 (`macos-latest`) to Linux x64 (`ubuntu-latest`) to eliminate costly runner minutes, while retaining cross-platform compilation matrix for macOS and Linux.

### Mask Regex Enhancement: `<dollar>` Placeholder & Regex Delimiter Support

- **`<dollar>` Placeholder**: Added automatic substitution of `<dollar>` with `$` in mask patterns. This allows defining regex end-of-string anchors without triggering shell or `.env` variable interpolation (e.g. `$()`, `${}`).
- **Slash Delimiters**: Patterns wrapped in `/.../` slashes (e.g. `ENCODEMASK="/.../"`) have their outer delimiters automatically stripped before compiling the regular expression.
- **Test Suite**: Added test case `test009` verifying end-to-end masking with `<dollar>` anchors and surrounding slashes.

### Security: Upgrade KDF from SHA-256 to Argon2id with Random Salts

- **Issue Mitigated**: Weak key derivation. Deriving an AES-256 key with a single SHA-256 pass allowed attackers to perform fast, cheap offline dictionary and brute-force attacks against encrypted values without work factors or salt protections.
- **Fix**: Upgraded key derivation to memory-hard **Argon2id**.
- **Payload Versioning**: Encrypted payloads now carry a version byte (`0x01`), a cryptographically secure 16-byte random salt per encrypted value, and a 12-byte random nonce before the AES-256-GCM ciphertext and authentication tag.
- **Compatibility**: Legacy v0 payloads encrypted with single-pass SHA-256 remain supported for decryption.

### Security: Memory Sanitization (Zeroize)

- **Issue Mitigated**: Lingering sensitive keys/passwords in memory after operations finish.
- **Fix**: Incorporated `zeroize` crate. Derived 32-byte cryptographic keys, raw plaintext byte buffers, and resolved password strings are explicitly wiped with zeroes (`Zeroizing` / `.zeroize()`) upon drop / completion.

### Security: Error Information Disclosure Hardening

- **Issue Mitigated**: Potential low-level crypto error leakage or side-channel hints (such as specific internal AEAD or UTF-8 parser errors) in error outputs.
- **Fix**: Unified and sanitized decryption failure messages to generic authentication error messages (`decryption failed (authentication check failed)` / `invalid base64 payload`).

### CLI & Release Enhancements

- **Dynamic Version Injection**: Version is now embedded into the binary at compile time via `build.rs` from git tags (or `SECURENV_VERSION` environment variable in CI).
- **Homepage in CLI Help**: Added GitHub repository homepage (`https://github.com/stopsopa/securenv`) and build version display to `--help`.
- **Release Documentation**: Standardized release download snippet to save binary directly as `securenv` and verify checksums against `securenv`.
