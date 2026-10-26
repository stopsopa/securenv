# securenv

Compact CLI tool to selectively encrypt and decrypt `.env` values using **AES-256-GCM** with **Argon2id** password-based key derivation (with per-value random salts and versioned payloads) while preserving file comments, whitespace, and formatting.

## Installation

Download the binary for your platform from the [GitHub Releases](https://github.com/stopsopa/securenv/releases) page. Each release contains direct `curl` / `wget` commands and SHA-256 checksums for each architecture.

Once downloaded, move the binary into your system `PATH`:

```sh
# Move downloaded binary to /usr/local/bin:
sudo cp securenv /usr/local/bin/securenv

# Make it executable:
sudo chmod a+x /usr/local/bin/securenv

# Verify installation:
securenv --help

# Bring to docker

# download linux arm
curl -fL -o securenv \
  https://github.com/stopsopa/securenv/releases/download/v1.2.0/securenv-linux-arm64

chmod a+x securenv

docker run --rm \
  -v "$(pwd)/securenv:/usr/local/bin/securenv:ro" \
  --entrypoint /usr/local/bin/securenv \
  gcr.io/distroless/cc-debian13:latest \
  --help

```

## Usage

```sh
# Encrypt matched variables from .env.tmp into .env
securenv encrypt .env.tmp .env --mask ENCODEMASK --password "supersecretpassword"

# Decrypt back to plaintext
securenv decrypt .env .env.tmp --mask ENCODEMASK --password "supersecretpassword"
```

### Password via Environment Variable

```sh
export MY_KEY="supersecretpassword"
securenv encrypt .env.tmp .env --mask ENCODEMASK --password-env MY_KEY
```

### Output to Stdout

```sh
securenv encrypt .env.tmp .env --mask ENCODEMASK --password "supersecretpassword" --stdout
```

### Custom Prefix

By default, encrypted values are prefixed with `securenv:` (e.g. `KEY="securenv:<base64>"`). You can override this using `--prefix`:

```sh
securenv encrypt .env.tmp .env --mask ENCODEMASK --password "supersecretpassword" --prefix "vault:"
```

# example mask 

```sh

ENCODEMASK="/(^PUBLIC_|^FIREBASE_|^NODE_API_|^(PROJECT_NAME|NODE_ENV|JWT_EXPIRE_SECONDS|HOST|VITE_PROXY)<dollar>)/"

```

In shells and `.env` files, a literal `$` can accidentally trigger parameter expansion or variable interpolation (like `$VAR`, `$()`, or `${}`). To prevent this, securenv automatically replaces all occurrences of `<dollar>` with `$` before compiling the regex pattern. Surrounding `/.../` slashes are also stripped automatically.

## Flags & Arguments

- `encrypt <source> [target]`: Encrypt matched values from source into target.
- `decrypt <source> [target]`: Decrypt matched values from source into target.
- `--mask <VAR>`: Name of the variable (in file or process environment) containing a regex pattern. Only variables matching this regex have their values transformed. If omitted, all variables are processed.
- `--password <PWD>`: Password string for encryption/decryption key derivation (AES-256-GCM via SHA-256).
- `--password-env <VAR>`: Name of an environment variable containing the password.
- `--prefix <PREFIX>`: Prefix marker for encrypted values (default: `securenv:`).
- `--stdout`: Output transformed content to stdout instead of target file. Target argument must not be supplied when using this flag.
- `--help`: Print compact usage information and exit.

## `##` Comment Encryption

Lines starting with `##` (optionally preceded by spaces/tabs) that contain non-whitespace content are encrypted and decrypted alongside matched env vars. The `##` marker and any whitespace before the payload stay in plaintext; only the payload is encrypted.

```sh
## this comment will be encrypted
##this too (no space)
##  and this (two spaces)
  ## leading indent preserved
### extra # treated as payload too

##         ← only whitespace after ## → left untouched
# regular single-hash comment   → always left untouched
```

Encrypted form keeps the exact prefix so the file remains valid:

```sh
## securenv:<base64>
```

## dev

See [DEV.md](DEV.md) for local development setup, toolchain requirements, testing, and release process.
