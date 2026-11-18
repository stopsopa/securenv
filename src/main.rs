use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Key, Nonce,
};
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use rand::RngCore;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{IsTerminal, Write},
    path::PathBuf,
    process,
    sync::atomic::{AtomicBool, Ordering},
};
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Crypto helpers
// ---------------------------------------------------------------------------

/// v1: Argon2id + salt, no associated data. Decrypt only.
const FORMAT_VERSION_ARGON2ID: u8 = 0x01;
/// v2: Argon2id + salt, variable name bound as AES-GCM associated data,
/// plaintext is the verbatim right-hand side of the assignment. Used for all new encryption.
const FORMAT_VERSION_ARGON2ID_AAD: u8 = 0x02;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

/// Associated data used for `##` comment payloads (a variable can never have this name).
const COMMENT_AAD: &[u8] = b"##";

/// Set once the legacy SHA-256 warning was printed, so it is printed only once per run.
static LEGACY_WARNED: AtomicBool = AtomicBool::new(false);

/// Derive a 32-byte AES-256 key from a password using Argon2id and a 16-byte salt.
/// Zeroizes key buffer on drop.
fn derive_key_argon2id(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, String> {
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut *key)
        .map_err(|e| format!("Argon2id key derivation error: {e}"))?;
    Ok(key)
}

/// Derive a 32-byte AES-256 key from a password using legacy SHA-256 (for backwards compatibility).
/// Zeroizes key buffer on drop.
fn derive_key_sha256(password: &str) -> Zeroizing<[u8; 32]> {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    Zeroizing::new(hasher.finalize().into())
}

// Build AES-256-GCM cipher from 32-byte key.
fn cipher_for(key: &[u8; 32]) -> Aes256Gcm {
    Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key))
}

/// Encrypt plaintext with AES-256-GCM using Argon2id key derivation.
/// `aad` (variable name) is authenticated but not stored: decryption under a
/// different variable name fails.
/// Returns base64-encoded:
/// [1 byte version = 0x02] || [16 bytes random salt] || [12 bytes random nonce] || [AES-GCM ciphertext + tag]
fn encrypt_value(plaintext: &str, password: &str, aad: &[u8]) -> Result<String, String> {
    let mut salt = [0u8; SALT_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);

    let key_bytes = derive_key_argon2id(password, &salt)?;
    let cipher = cipher_for(&key_bytes);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext.as_bytes(),
                aad,
            },
        )
        .map_err(|_| "AES-GCM encryption failed".to_string())?;

    let mut combined = Vec::with_capacity(1 + SALT_LEN + NONCE_LEN + ciphertext.len());
    combined.push(FORMAT_VERSION_ARGON2ID_AAD);
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(&combined))
}

// AES-256-GCM decrypt, None when authentication fails. Plaintext buffer zeroized on drop.
fn aes_gcm_decrypt(
    key: &[u8; 32],
    nonce: &[u8],
    ciphertext: &[u8],
    aad: &[u8],
) -> Option<Zeroizing<Vec<u8>>> {
    cipher_for(key)
        .decrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .ok()
        .map(Zeroizing::new)
}

/// Decrypt a base64-encoded payload with AES-256-GCM.
/// Supports version 0x02 (Argon2id + salt + associated data), version 0x01 (Argon2id + salt)
/// and legacy v0 (raw nonce || ciphertext via SHA-256).
///
/// Legacy v0 has no version byte, so its random nonce can start with 0x01 / 0x02.
/// That is why legacy is always tried as a fallback when versioned attempt fails.
///
/// Returns (plaintext, verbatim). `verbatim` is true for 0x02 payloads where plaintext
/// is the exact right-hand side of the assignment (quotes included). For older payloads
/// plaintext is the unquoted value.
fn decrypt_value(encoded: &str, password: &str, aad: &[u8]) -> Result<(String, bool), String> {
    const HEADER_LEN: usize = 1 + SALT_LEN + NONCE_LEN;

    let combined = BASE64
        .decode(encoded)
        .map_err(|_| "invalid base64 payload".to_string())?;

    let version = combined.first().copied();
    let is_v2 = version == Some(FORMAT_VERSION_ARGON2ID_AAD);
    let is_v1 = version == Some(FORMAT_VERSION_ARGON2ID);

    let mut plaintext: Option<(Zeroizing<Vec<u8>>, bool)> = None;

    if (is_v1 || is_v2) && combined.len() >= HEADER_LEN + TAG_LEN {
        let key_bytes = derive_key_argon2id(password, &combined[1..1 + SALT_LEN])?;
        let versioned_aad: &[u8] = if is_v2 { aad } else { &[] };
        plaintext = aes_gcm_decrypt(
            &key_bytes,
            &combined[1 + SALT_LEN..HEADER_LEN],
            &combined[HEADER_LEN..],
            versioned_aad,
        )
        .map(|bytes| (bytes, is_v2));
    }

    // Fallback: Legacy v0 payload (12-byte nonce || ciphertext) using SHA-256 key derivation
    if plaintext.is_none() && combined.len() >= NONCE_LEN + TAG_LEN {
        let key_bytes = derive_key_sha256(password);
        let no_aad: &[u8] = &[];
        plaintext = aes_gcm_decrypt(
            &key_bytes,
            &combined[..NONCE_LEN],
            &combined[NONCE_LEN..],
            no_aad,
        )
        .map(|bytes| (bytes, false));

        if plaintext.is_some() && !LEGACY_WARNED.swap(true, Ordering::Relaxed) {
            eprintln!(
                "securenv warning: source contains values in legacy SHA-256 format (weak key derivation) - encrypt the file again to upgrade"
            );
        }
    }

    let (bytes, verbatim) = plaintext
        .ok_or_else(|| "decryption failed (authentication check failed)".to_string())?;

    let text = String::from_utf8(bytes.to_vec())
        .map_err(|_| "decrypted value is not valid UTF-8".to_string())?;

    Ok((text, verbatim))
}

// ---------------------------------------------------------------------------
// .env file parsing
// ---------------------------------------------------------------------------

// Split trailing `\r` off a line so CRLF files keep their line endings.
fn split_cr(line: &str) -> (&str, &'static str) {
    match line.strip_suffix('\r') {
        Some(body) => (body, "\r"),
        None => (line, ""),
    }
}

/// Strips surrounding quote pair (double or single) from the value.
fn unquote(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &value[1..value.len() - 1];
        }
    }
    value
}

/// Split a single .env file line into parts of key=value assignment.
/// Returns (lhs, key, rhs) or None for comments / blank lines:
/// - lhs: everything up to and including `=` (indentation and `export ` kept), copied to output as-is
/// - key: variable name without `export ` prefix, used for mask matching
/// - rhs: everything after `=`, verbatim (quotes, inline comments, whitespace)
///
/// Examples:
///   `KEY="val"`            -> (`KEY=`, `KEY`, `"val"`)
///   `export KEY=val # x`   -> (`export KEY=`, `KEY`, `val # x`)
///   `  KEY = 'val'`        -> (`  KEY =`, `KEY`, ` 'val'`)
fn split_assignment(line: &str) -> Option<(&str, &str, &str)> {
    let trimmed = line.trim_start();

    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let eq_pos = line.find('=')?;
    let name = line[..eq_pos].trim();

    let key = match name.strip_prefix("export") {
        Some(rest) if rest.starts_with([' ', '\t']) => rest.trim_start(),
        _ => name,
    };

    if key.is_empty() {
        return None;
    }

    Some((&line[..=eq_pos], key, &line[eq_pos + 1..]))
}

// Returns quote character if value starts with one (after optional whitespace).
fn open_quote(rhs: &str) -> Option<char> {
    rhs.trim_start().chars().next().filter(|c| *c == '"' || *c == '\'')
}

/// Check if `text` contains closing `quote`. Backslash escapes next character
/// inside double quotes (`\"`), single quotes have no escapes.
fn quote_closed(text: &str, quote: char) -> bool {
    let mut escaped = false;
    for c in text.chars() {
        if escaped {
            escaped = false;
        } else if c == '\\' && quote == '"' {
            escaped = true;
        } else if c == quote {
            return true;
        }
    }
    false
}

/// One logical element of .env file.
enum Item<'a> {
    /// Single-# comment, blank line or anything not recognised – copied unchanged.
    Other(&'a str),
    /// Encryptable `##` comment line.
    Comment {
        comment: DoubleHashComment<'a>,
        cr: &'static str,
    },
    /// key=value assignment. Spans multiple lines when quoted value is not closed on first line.
    Assign {
        lhs: &'a str,
        key: &'a str,
        /// verbatim right-hand side (without trailing `\r` of last line)
        rhs: String,
        /// original text of all lines of this assignment
        raw: String,
        cr: &'static str,
    },
}

/// Parse whole .env file content into items.
/// Quoted value not closed on its line continues on next lines until closing quote
/// (multi-line values like PEM keys or JSON). Never-closed quote is an error –
/// better fail than leave part of a secret unencrypted.
fn parse_items(content: &str) -> Result<Vec<Item<'_>>, String> {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut items = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let line_no = i + 1;
        i += 1;

        let (body, cr) = split_cr(line);

        if let Some(comment) = parse_double_hash_comment(body) {
            items.push(Item::Comment { comment, cr });
            continue;
        }

        let Some((lhs, key, first_rhs)) = split_assignment(body) else {
            items.push(Item::Other(line));
            continue;
        };

        let mut raw = line.to_string();

        if let Some(quote) = open_quote(first_rhs) {
            let mut closed = quote_closed(&first_rhs.trim_start()[1..], quote);
            while !closed {
                if i >= lines.len() {
                    return Err(format!(
                        "unterminated {quote} quoted value for var '{key}' starting at line {line_no}"
                    ));
                }
                let next = lines[i];
                i += 1;
                raw.push('\n');
                raw.push_str(next);
                closed = quote_closed(next, quote);
            }
        }

        let (raw_body, cr) = split_cr(&raw);
        let rhs = raw_body[lhs.len()..].to_string();

        items.push(Item::Assign {
            lhs,
            key,
            rhs,
            raw,
            cr,
        });
    }

    Ok(items)
}

/// Build a key→value STORAGE map from parsed .env items (values without surrounding quotes).
fn build_storage(items: &[Item]) -> HashMap<String, String> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Assign { key, rhs, .. } => {
                Some((key.to_string(), unquote(rhs.trim()).to_string()))
            }
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Argument resolution helpers
// ---------------------------------------------------------------------------

/// Resolve password from --password or --password-env.
/// Returned string is zeroized on drop.
fn resolve_password(
    password: &Option<String>,
    password_env: &Option<String>,
) -> Result<Zeroizing<String>, String> {
    if let Some(pwd) = password {
        return Ok(Zeroizing::new(pwd.clone()));
    }

    if let Some(env_var) = password_env {
        let val = std::env::var(env_var)
            .map_err(|_| format!("password env var '{env_var}' is not set"))?;
        if val.is_empty() {
            return Err(format!("password env var '{env_var}' is set but empty"));
        }
        return Ok(Zeroizing::new(val));
    }

    Err("either --password or --password-env must be provided".to_string())
}

/// Resolve mask regex: look in STORAGE first, then in process environment.
/// Automatically replaces any `<dollar>` placeholder with `$` to avoid shell parameter expansion issues.
/// Also strips leading and trailing `/` delimiters if present (e.g. `/^PATTERN$/` -> `^PATTERN$`).
fn resolve_mask(mask_var: &str, storage: &HashMap<String, String>) -> Result<Regex, String> {
    let raw_pattern = storage
        .get(mask_var)
        .cloned()
        .or_else(|| std::env::var(mask_var).ok())
        .ok_or_else(|| {
            format!("mask var '{mask_var}' not found in source file or process environment")
        })?;

    let replaced = raw_pattern.replace("<dollar>", "$");
    let trimmed = replaced.trim();
    let final_pattern = if trimmed.len() >= 2 && trimmed.starts_with('/') && trimmed.ends_with('/') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };

    Regex::new(final_pattern).map_err(|e| {
        format!("invalid regex in mask var '{mask_var}' (value: '{raw_pattern}'): {e}")
    })
}

// ---------------------------------------------------------------------------
// ## comment encryption helpers
// ---------------------------------------------------------------------------

/// Represents a parsed `##` comment line broken into its parts.
struct DoubleHashComment<'a> {
    /// Everything up to and including `##` plus any leading whitespace before
    /// the first non-whitespace payload character, e.g. `"  ## "`.
    prefix: &'a str,
    /// The non-whitespace payload that should be encrypted/decrypted.
    payload: &'a str,
}

/// Try to parse a line as an encryptable `##` comment.
///
/// Rules:
/// - Line (after stripping leading spaces/tabs) must start with `##`.
/// - There must be at least one non-whitespace character after `##`
///   (possibly separated by spaces/tabs). Lines that are only `##` + spaces
///   are ignored (returned as `None`).
/// - Extra `#` characters immediately after `##` (no whitespace separator)
///   are treated as part of the payload to encrypt.
///
/// Returns `Some(DoubleHashComment)` when the line should be encrypted/decrypted,
/// `None` when the line must be left untouched.
fn parse_double_hash_comment(line: &str) -> Option<DoubleHashComment<'_>> {
    // Find where the line content starts (skip leading spaces/tabs)
    let content_start = line
        .char_indices()
        .find(|(_, c)| *c != ' ' && *c != '\t')
        .map(|(i, _)| i)
        .unwrap_or(line.len());

    let rest = &line[content_start..];

    // Must start with exactly `##`
    if !rest.starts_with("##") {
        return None;
    }

    // Everything after `##`
    let after_hashes = &rest[2..];

    // Find the first non-whitespace character in `after_hashes` — that is the
    // start of the payload. The whitespace between `##` and the payload is
    // kept in plaintext as part of the prefix.
    let payload_start_in_after = after_hashes
        .char_indices()
        .find(|(_, c)| *c != ' ' && *c != '\t')
        .map(|(i, _)| i);

    let payload_start_in_after = match payload_start_in_after {
        // No non-whitespace content → ignore this line
        None => return None,
        Some(i) => i,
    };

    // `prefix` = everything from line start up to (but not including) the payload
    let prefix_len = content_start + 2 + payload_start_in_after;
    let prefix = &line[..prefix_len];
    let payload = &line[prefix_len..];

    Some(DoubleHashComment { prefix, payload })
}

// ---------------------------------------------------------------------------
// Core processing
// ---------------------------------------------------------------------------

enum Mode {
    Encrypt,
    Decrypt,
}

/// Process an env file: encrypt or decrypt values matched by the mask regex,
/// and also encrypt/decrypt `##` comment lines that contain non-whitespace content.
/// Returns the transformed file content as a String, preserving all formatting.
///
/// Encrypt: whole right-hand side of assignment (quotes, inline comment, all lines of
/// multi-line value) is encrypted verbatim and replaced with `"<prefix><base64>"`.
/// Decrypt: right-hand side is restored exactly as it was before encryption.
fn process_file(
    mode: &Mode,
    source: &PathBuf,
    mask: &Option<String>,
    password: &str,
    prefix: &str,
) -> Result<String, String> {
    let content = fs::read_to_string(source)
        .map_err(|e| format!("cannot read source file '{}': {e}", source.display()))?;

    let items = parse_items(&content)?;

    let storage = build_storage(&items);

    let regex_opt: Option<Regex> = match mask {
        Some(mask_var) => Some(resolve_mask(mask_var, &storage)?),
        None => None,
    };

    let mut output_lines: Vec<String> = Vec::new();

    for item in items {
        let new_line = match item {
            // Single-# comment or blank line – copy unchanged
            Item::Other(line) => line.to_string(),

            // --- `##` comment lines: always processed regardless of mask ---
            Item::Comment { comment: dh, cr } => match mode {
                Mode::Encrypt => {
                    let b64 = encrypt_value(dh.payload, password, COMMENT_AAD)
                        .map_err(|e| format!("encrypt failed for ## comment: {e}"))?;
                    format!("{}{}{}{}", dh.prefix, prefix, b64, cr)
                }
                Mode::Decrypt => match dh.payload.strip_prefix(prefix) {
                    Some(encoded) => {
                        let (plain, _) = decrypt_value(encoded, password, COMMENT_AAD)
                            .map_err(|e| {
                                format!(
                                    "decrypt failed for ## comment (payload: '{}'): {e}",
                                    dh.payload
                                )
                            })?;
                        format!("{}{}{}", dh.prefix, plain, cr)
                    }
                    // Payload does not carry the prefix – copy line unchanged
                    None => format!("{}{}{}", dh.prefix, dh.payload, cr),
                },
            },

            // --- env var assignment lines ---
            Item::Assign {
                lhs,
                key,
                rhs,
                raw,
                cr,
            } => {
                let should_transform = match &regex_opt {
                    Some(re) => re.is_match(key),
                    None => true,
                };

                if should_transform {
                    match mode {
                        Mode::Encrypt => {
                            // raw value deliberately not printed – it is a plaintext secret
                            let b64 = encrypt_value(&rhs, password, key.as_bytes())
                                .map_err(|e| format!("encrypt failed for var '{key}': {e}"))?;
                            format!("{lhs}\"{prefix}{b64}\"{cr}")
                        }
                        Mode::Decrypt => match unquote(rhs.trim()).strip_prefix(prefix) {
                            Some(encoded) => {
                                let (plain, verbatim) =
                                    decrypt_value(encoded, password, key.as_bytes()).map_err(
                                        |e| {
                                            format!(
                                                "decrypt failed for var '{key}' (raw value: '{}'): {e}",
                                                rhs.trim()
                                            )
                                        },
                                    )?;
                                if verbatim {
                                    format!("{lhs}{plain}{cr}")
                                } else {
                                    // payloads older than v2 hold unquoted value
                                    format!("{lhs}\"{plain}\"{cr}")
                                }
                            }
                            // Value does not carry the prefix – copy line unchanged
                            None => raw,
                        },
                    }
                } else {
                    // Not matched by mask – copy line unchanged
                    raw
                }
            }
        };
        output_lines.push(new_line);
    }

    // split('\n') + join("\n") keeps trailing newline (or lack of it) of the original
    Ok(output_lines.join("\n"))
}

/// Write `output` to file, overwriting it.
/// With `private` the file gets 0600 permissions (decrypted secrets).
fn write_file(path: &PathBuf, output: &str, private: bool) -> std::io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        if private {
            options.mode(0o600);
        }
    }

    let mut file = options.open(path)?;

    // mode() above works only for new files, fix permissions of already existing one
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if private {
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
    }

    #[cfg(not(unix))]
    let _ = private;

    file.write_all(output.as_bytes())
}

/// Write output to stdout or to a target file (with parent-dir existence check).
fn write_output(
    output: &str,
    target: &Option<PathBuf>,
    use_stdout: bool,
    private: bool,
) -> Result<(), String> {
    if use_stdout {
        print!("{output}");
        return Ok(());
    }

    let target_path = target.as_ref().unwrap();

    if let Some(parent) = target_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(format!(
                "target file parent directory '{}' does not exist",
                parent.display()
            ));
        }
    }

    write_file(target_path, output, private)
        .map_err(|e| format!("cannot write to target file '{}': {e}", target_path.display()))
}

/// Shared argument handling for both Encrypt and Decrypt subcommands.
fn handle_command(mode: Mode, args: CommandArgs) -> Result<(), String> {
    if args.stdout && args.target.is_some() {
        return Err(
            "--stdout cannot be used together with a target file argument".to_string(),
        );
    }
    if !args.stdout && args.target.is_none() {
        return Err(
            "target file argument is required when --stdout is not used".to_string(),
        );
    }

    let pwd = resolve_password(&args.password, &args.password_env)?;
    let output = Zeroizing::new(process_file(
        &mode,
        &args.source,
        &args.mask,
        &pwd,
        &args.prefix,
    )?);

    // decrypted file holds plaintext secrets – owner-only permissions
    let private = matches!(mode, Mode::Decrypt);
    write_output(&output, &args.target, args.stdout, private)
}

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

/// Note about recommended way of passing the password.
/// Red when `color` is true (stdout is interactive terminal).
fn help_recommendation(color: bool) -> String {
    let (red, reset) = if color {
        ("\x1b[31m", "\x1b[0m")
    } else {
        ("", "")
    };
    format!(
        "{red}RECOMMENDED: pass the password with --password-env <VAR>.\n--password <PWD> exposes the password in `ps` output and shell history.{reset}"
    )
}

// Short help (-h)
fn help_about(color: bool) -> String {
    format!(
        "securenv — AES-256-GCM .env value encryptor (version: {})\nHomepage: https://github.com/stopsopa/securenv\n\n{}",
        env!("SECURENV_BUILD_VERSION"),
        help_recommendation(color)
    )
}

// Long help (--help)
fn help_long_about(color: bool) -> String {
    let (red, reset) = if color {
        ("\x1b[31m", "\x1b[0m")
    } else {
        ("", "")
    };
    format!(
        concat!(
            "securenv — AES-256-GCM .env value encryptor\n",
            "Version:  {version}\n",
            "Homepage: https://github.com/stopsopa/securenv\n\n",
            "Selectively encrypts and decrypts .env file values using AES-256-GCM and Argon2id,\n",
            "preserving comments, whitespace, and formatting.\n\n",
            "Usage:\n",
            "  securenv encrypt <source> [target] [OPTIONS]\n",
            "  securenv decrypt <source> [target] [OPTIONS]\n\n",
            "Options:\n",
            "  --mask <VAR>          name of env var (in file or process env) holding regex mask\n",
            "  --password-env <VAR>  name of env var holding the password {red}(recommended){reset}\n",
            "  --password <PWD>      encryption password {red}(visible in ps / shell history){reset}\n",
            "  --prefix <PREFIX>     encrypted value prefix (default: securenv:)\n",
            "  --stdout              write result to stdout; target must NOT be provided\n",
            "  -V, --version         print version and exit\n",
            "  -h, --help            print this help and exit\n\n",
            "{recommendation}",
        ),
        version = env!("SECURENV_BUILD_VERSION"),
        red = red,
        reset = reset,
        recommendation = help_recommendation(color),
    )
}

#[derive(Parser)]
#[command(name = "securenv", version = env!("SECURENV_BUILD_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

// Arguments shared by encrypt and decrypt subcommands.
// (regular comment on purpose: doc comment here would replace subcommand description in help)
#[derive(Args)]
struct CommandArgs {
    /// Source .env file to read
    source: PathBuf,
    /// Target file to write (omit when --stdout is used)
    target: Option<PathBuf>,
    /// Name of env var (in source file or process env) holding the regex mask
    #[arg(long)]
    mask: Option<String>,
    /// Encryption password (visible in ps / shell history - prefer --password-env)
    #[arg(long)]
    password: Option<String>,
    /// Name of env var holding the encryption password (recommended)
    #[arg(long = "password-env")]
    password_env: Option<String>,
    /// Prefix that marks encrypted values (default: "securenv:")
    #[arg(long, default_value = "securenv:")]
    prefix: String,
    /// Write output to stdout instead of target file
    #[arg(long)]
    stdout: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Encrypt matched env var values in <source> and write result to <target>
    Encrypt(CommandArgs),
    /// Decrypt matched env var values in <source> and write result to <target>
    Decrypt(CommandArgs),
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn run() -> Result<(), String> {
    // color only when help is printed to interactive terminal (and NO_COLOR is not set)
    let color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let recommendation = help_recommendation(color);

    let matches = Cli::command()
        .about(help_about(color))
        .long_about(help_long_about(color))
        .mut_subcommand("encrypt", |c| c.after_help(recommendation.clone()))
        .mut_subcommand("decrypt", |c| c.after_help(recommendation.clone()))
        .get_matches();

    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    match cli.command {
        Commands::Encrypt(args) => handle_command(Mode::Encrypt, args),
        Commands::Decrypt(args) => handle_command(Mode::Decrypt, args),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("securenv error: {e}");
        process::exit(1);
    }
}
