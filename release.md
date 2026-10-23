# Release Notes

### Security: Upgrade KDF from SHA-256 to Argon2id with Random Salts

- **Issue Mitigated**: Weak key derivation. Deriving an AES-256 key with a single SHA-256 pass allowed attackers to perform fast, cheap offline dictionary and brute-force attacks against encrypted values without work factors or salt protections.
- **Fix**: Upgraded key derivation to memory-hard **Argon2id**.
- **Payload Versioning**: Encrypted payloads now carry a version byte (`0x01`), a cryptographically secure 16-byte random salt per encrypted value, and a 12-byte random nonce before the AES-256-GCM ciphertext and authentication tag.
- **Compatibility**: Legacy v0 payloads encrypted with single-pass SHA-256 remain supported for decryption.
