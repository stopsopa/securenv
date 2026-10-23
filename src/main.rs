use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use clap::{Parser, Subcommand};
use rand::RngCore;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, path::PathBuf, process};
use zeroize::{Zeroize, Zeroizing};

// ---------------------------------------------------------------------------
// Crypto helpers
// ---------------------------------------------------------------------------

const FORMAT_VERSION_ARGON2ID: u8 = 0x01;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

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

/// Encrypt plaintext with AES-256-GCM using Argon2id key derivation.
/// Returns base64-encoded:
/// [1 byte version = 0x01] || [16 bytes random salt] || [12 bytes random nonce] || [AES-GCM ciphertext + tag]
fn encrypt_value(plaintext: &str, password: &str) -> Result<String, String> {
    let mut salt = [0u8; SALT_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);

    let key_bytes = derive_key_argon2id(password, &salt)?;
    let key = Key::<Aes256Gcm>::from_slice(&*key_bytes);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| "AES-GCM encryption failed".to_string())?;

    let mut combined = Vec::with_capacity(1 + SALT_LEN + NONCE_LEN + ciphertext.len());
    combined.push(FORMAT_VERSION_ARGON2ID);
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(&combined))
}

/// Decrypt a base64-encoded payload with AES-256-GCM.
/// Supports both version 0x01 (Argon2id + salt) and legacy v0 (raw nonce || ciphertext via SHA-256).
fn decrypt_value(encoded: &str, password: &str) -> Result<String, String> {
    let combined = BASE64
        .decode(encoded)
        .map_err(|_| "invalid base64 payload".to_string())?;

    // Check if payload starts with version byte 0x01 (Argon2id)
    if !combined.is_empty() && combined[0] == FORMAT_VERSION_ARGON2ID {
        const HEADER_LEN: usize = 1 + SALT_LEN + NONCE_LEN;
        if combined.len() < HEADER_LEN {
            return Err("encrypted payload too short for Argon2id format".to_string());
        }

        let salt = &combined[1..1 + SALT_LEN];
        let nonce_bytes = &combined[1 + SALT_LEN..HEADER_LEN];
        let ciphertext = &combined[HEADER_LEN..];

        let key_bytes = derive_key_argon2id(password, salt)?;
        let key = Key::<Aes256Gcm>::from_slice(&*key_bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(nonce_bytes);

        let mut plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| "decryption failed (authentication check failed)".to_string())?;

        let res = String::from_utf8(plaintext.clone())
            .map_err(|_| "decrypted value is not valid UTF-8".to_string());
        plaintext.zeroize();
        res
    } else {
        // Fallback: Legacy v0 payload (12-byte nonce || ciphertext) using SHA-256 key derivation
        if combined.len() < NONCE_LEN {
            return Err("encrypted payload too short (need at least 12 bytes for nonce)".to_string());
        }

        let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        let key_bytes = derive_key_sha256(password);
        let key = Key::<Aes256Gcm>::from_slice(&*key_bytes);
        let cipher = Aes256Gcm::new(key);

        let mut plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| "decryption failed (authentication check failed)".to_string())?;

        let res = String::from_utf8(plaintext.clone())
            .map_err(|_| "decrypted value is not valid UTF-8".to_string());
        plaintext.zeroize();
        res
    }
}

// ---------------------------------------------------------------------------
// .env file parsing
// ---------------------------------------------------------------------------

/// Parse a single .env file line as a key=value assignment.
/// Strips surrounding quote pair (double or single) from the value.
/// Returns (key, raw_value_without_quotes) or None for comments / blank lines.
fn parse_assignment(line: &str) -> Option<(&str, String)> {
    let trimmed = line.trim();

    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let eq_pos = trimmed.find('=')?;
    let key = trimmed[..eq_pos].trim();

    if key.is_empty() {
        return None;
    }

    let rest = &trimmed[eq_pos + 1..];

    // Strip surrounding quote pair
    let raw_value = if rest.len() >= 2 {
        let bytes = rest.as_bytes();
        let first = bytes[0];
        let last = bytes[rest.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            rest[1..rest.len() - 1].to_string()
        } else {
            rest.to_string()
        }
    } else {
        rest.to_string()
    };

    Some((key, raw_value))
}

/// Build a key→value STORAGE map from .env file content.
fn build_storage(content: &str) -> HashMap<String, String> {
    content
        .lines()
        .filter_map(parse_assignment)
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

// ---------------------------------------------------------------------------
// Argument resolution helpers
// ---------------------------------------------------------------------------

/// Resolve password from --password or --password-env.
fn resolve_password(
    password: &Option<String>,
    password_env: &Option<String>,
) -> Result<String, String> {
    if let Some(pwd) = password {
        return Ok(pwd.clone());
    }

    if let Some(env_var) = password_env {
        let val = std::env::var(env_var)
            .map_err(|_| format!("password env var '{env_var}' is not set"))?;
        if val.is_empty() {
            return Err(format!("password env var '{env_var}' is set but empty"));
        }
        return Ok(val);
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
// Core processing
// ---------------------------------------------------------------------------

enum Mode {
    Encrypt,
    Decrypt,
}

/// Process an env file: encrypt or decrypt values matched by the mask regex.
/// Returns the transformed file content as a String, preserving all formatting.
fn process_file(
    mode: &Mode,
    source: &PathBuf,
    mask: &Option<String>,
    password: &str,
    prefix: &str,
) -> Result<String, String> {
    let content = fs::read_to_string(source)
        .map_err(|e| format!("cannot read source file '{}': {e}", source.display()))?;

    let storage = build_storage(&content);

    let regex_opt: Option<Regex> = match mask {
        Some(mask_var) => Some(resolve_mask(mask_var, &storage)?),
        None => None,
    };

    let mut output_lines: Vec<String> = Vec::new();

    for line in content.lines() {
        if let Some((key_name, raw_value)) = parse_assignment(line) {
            let should_transform = match &regex_opt {
                Some(re) => re.is_match(key_name),
                None => true,
            };

            if should_transform {
                let new_value = match mode {
                    Mode::Encrypt => {
                        let b64 = encrypt_value(&raw_value, password).map_err(|e| {
                            format!(
                                "encrypt failed for var '{key_name}' (raw value: '{raw_value}'): {e}"
                            )
                        })?;
                        format!("{prefix}{b64}")
                    }
                    Mode::Decrypt => {
                        if raw_value.starts_with(prefix) {
                            let encoded = &raw_value[prefix.len()..];
                            decrypt_value(encoded, password).map_err(|e| {
                                format!(
                                    "decrypt failed for var '{key_name}' (raw value: '{raw_value}'): {e}"
                                )
                            })?
                        } else {
                            // Value does not carry the prefix – copy line unchanged
                            output_lines.push(line.to_string());
                            continue;
                        }
                    }
                };
                output_lines.push(format!("{key_name}=\"{new_value}\""));
            } else {
                // Not matched by mask – copy line unchanged
                output_lines.push(line.to_string());
            }
        } else {
            // Comment or blank line – copy unchanged
            output_lines.push(line.to_string());
        }
    }

    // Preserve trailing newline if the original had one
    let mut result = output_lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }

    Ok(result)
}

/// Write output to stdout or to a target file (with parent-dir existence check).
fn write_output(output: &str, target: &Option<PathBuf>, use_stdout: bool) -> Result<(), String> {
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

    fs::write(target_path, output)
        .map_err(|e| format!("cannot write to target file '{}': {e}", target_path.display()))
}

/// Shared argument handling for both Encrypt and Decrypt subcommands.
fn handle_command(
    mode: Mode,
    source: PathBuf,
    target: Option<PathBuf>,
    mask: Option<String>,
    password: Option<String>,
    password_env: Option<String>,
    prefix: String,
    use_stdout: bool,
) -> Result<(), String> {
    if use_stdout && target.is_some() {
        return Err(
            "--stdout cannot be used together with a target file argument".to_string(),
        );
    }
    if !use_stdout && target.is_none() {
        return Err(
            "target file argument is required when --stdout is not used".to_string(),
        );
    }

    let mut pwd = resolve_password(&password, &password_env)?;
    let output = process_file(&mode, &source, &mask, &pwd, &prefix)?;
    pwd.zeroize();
    write_output(&output, &target, use_stdout)
}

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "securenv",
    version = env!("SECURENV_BUILD_VERSION"),
    about = concat!(
        "securenv — AES-256-GCM .env value encryptor (version: ",
        env!("SECURENV_BUILD_VERSION"),
        ")\nHomepage: https://github.com/stopsopa/securenv"
    ),
    long_about = concat!(
        "securenv — AES-256-GCM .env value encryptor\n",
        "Version:  ",
        env!("SECURENV_BUILD_VERSION"),
        "\nHomepage: https://github.com/stopsopa/securenv\n\n",
        "Selectively encrypts and decrypts .env file values using AES-256-GCM and Argon2id,\n",
        "preserving comments, whitespace, and formatting.\n\n",
        "Usage:\n",
        "  securenv encrypt <source> [target] [OPTIONS]\n",
        "  securenv decrypt <source> [target] [OPTIONS]\n\n",
        "Options:\n",
        "  --mask <VAR>          name of env var (in file or process env) holding regex mask\n",
        "  --password <PWD>      encryption password\n",
        "  --password-env <VAR>  name of env var holding the password\n",
        "  --prefix <PREFIX>     encrypted value prefix (default: securenv:)\n",
        "  --stdout              write result to stdout; target must NOT be provided\n",
        "  -V, --version         print version and exit\n",
        "  -h, --help            print this help and exit",
    )
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encrypt matched env var values in <source> and write result to <target>
    Encrypt {
        /// Source .env file to read
        source: PathBuf,
        /// Target file to write (omit when --stdout is used)
        target: Option<PathBuf>,
        /// Name of env var (in source file or process env) holding the regex mask
        #[arg(long)]
        mask: Option<String>,
        /// Encryption password
        #[arg(long)]
        password: Option<String>,
        /// Name of env var holding the encryption password
        #[arg(long = "password-env")]
        password_env: Option<String>,
        /// Prefix prepended to every encrypted value (default: "securenv:")
        #[arg(long, default_value = "securenv:")]
        prefix: String,
        /// Write output to stdout instead of target file
        #[arg(long)]
        stdout: bool,
    },
    /// Decrypt matched env var values in <source> and write result to <target>
    Decrypt {
        /// Source .env file to read
        source: PathBuf,
        /// Target file to write (omit when --stdout is used)
        target: Option<PathBuf>,
        /// Name of env var (in source file or process env) holding the regex mask
        #[arg(long)]
        mask: Option<String>,
        /// Encryption password
        #[arg(long)]
        password: Option<String>,
        /// Name of env var holding the encryption password
        #[arg(long = "password-env")]
        password_env: Option<String>,
        /// Prefix that marks encrypted values (default: "securenv:")
        #[arg(long, default_value = "securenv:")]
        prefix: String,
        /// Write output to stdout instead of target file
        #[arg(long)]
        stdout: bool,
    },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encrypt {
            source,
            target,
            mask,
            password,
            password_env,
            prefix,
            stdout,
        } => handle_command(
            Mode::Encrypt,
            source,
            target,
            mask,
            password,
            password_env,
            prefix,
            stdout,
        ),
        Commands::Decrypt {
            source,
            target,
            mask,
            password,
            password_env,
            prefix,
            stdout,
        } => handle_command(
            Mode::Decrypt,
            source,
            target,
            mask,
            password,
            password_env,
            prefix,
            stdout,
        ),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("securenv error: {e}");
        process::exit(1);
    }
}
