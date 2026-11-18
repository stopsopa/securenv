# Release Notes

### Security: Payload Format v2, Verbatim Values, Multi-line Values

- **Breaking**: files encrypted by this version (payload version byte `0x02`) cannot be decrypted by older binaries. Files encrypted by older versions (v1 Argon2id, legacy v0 SHA-256) still decrypt.
- **Multi-line values**: quoted value spanning multiple lines (PEM key, JSON) is now encrypted as a whole. Before only the first line was encrypted and the rest stayed in plaintext. Never-closed quote is now a clear error.
- **`export KEY=value`**: mask is matched against `KEY` (before it was matched against `export KEY`, so masked vars were silently left unencrypted). `export ` prefix and indentation are preserved.
- **Verbatim right-hand side**: everything after `=` (quote style, inline comment, whitespace) is encrypted and restored byte for byte. Before decrypt always produced `KEY="value"`, which changed meaning of single-quoted, unquoted and inline-commented values.
- **Value bound to variable name**: variable name is AES-GCM associated data. Swapping encrypted values between variables (or moving a value under another name) makes decrypt fail. `##` comments are bound to a fixed `##` label.
- **Legacy detection fix**: legacy v0 payload whose random nonce started with byte `0x01` (1 in 256) was treated as Argon2id and could not be decrypted. Legacy is now tried as fallback.
- **Legacy warning**: decrypting legacy SHA-256 payload prints one warning on stderr asking to encrypt the file again.
- **File permissions**: decrypted target file is written with `0600` permissions.
- **No plaintext in errors**: encrypt errors no longer print the raw plaintext value.
- **Line endings**: CRLF line endings are preserved.
- **Zeroize**: password and output buffer are wiped also when command ends with error.
- **Help**: `--help` marks `--password-env` as recommended and warns that `--password` is visible in `ps` / shell history (in red when stdout is an interactive terminal, plain when piped or `NO_COLOR` is set).
- **README**: `--password-env` is the documented default, fixed `--stdout` example and stale SHA-256 mention.
- **Test Suite**: added `test011` (verbatim values, `export`, multi-line, permissions, unterminated quote) and `test012` (swapped values rejected).

### Feature: `##` Comment Encryption in `.env` Files

- Lines starting with `##` (with optional leading spaces/tabs) that have non-whitespace content are now encrypted alongside matched env vars.
- `##` marker and any whitespace before the payload stay plaintext; only payload is encrypted. Format: `##<spaces><prefix><base64>`.
- Extra `#` immediately after `##` (e.g. `###`) is treated as payload. Lines with only `##` + whitespace are ignored.
- Round-trip safe. Added `test010` covering all variants and full diff round-trip.

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
