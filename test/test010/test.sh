#!/usr/bin/env bash
set -euo pipefail

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd "${DIR}/../.." && pwd)"

source "${ROOT}/bash/detect_arch.sh"
BIN="${SECURENV_BIN}"

if [[ ! -f "${BIN}" ]]; then
  echo "${0} error: binary not found at >${BIN}<"
  exit 1
fi

# ---------------------------------------------------------------------------
# test010a — encrypt
# ---------------------------------------------------------------------------

CMD_ENC="./bin/${ARCH_DIR}/securenv encrypt test/test010/.env.tmp test/test010/.env --mask ENCODEMASK --password \"testpwd010\""
(cd "${ROOT}" && eval "${CMD_ENC}")
STATUS_ENC=$?

if [[ "${STATUS_ENC}" -ne 0 ]]; then
  echo "${0} error: encrypt failed with exit code >${STATUS_ENC}<"
  exit 1
fi

if [[ ! -f "${DIR}/.env" ]]; then
  echo "${0} error: output file >${DIR}/.env< was not created"
  exit 1
fi

# SECRET_ vars must be encrypted
if ! grep -q 'SECRET_TOKEN="securenv:' "${DIR}/.env"; then
  echo "${0} error: SECRET_TOKEN was not encrypted"
  exit 1
fi

if ! grep -q 'SECRET_KEY="securenv:' "${DIR}/.env"; then
  echo "${0} error: SECRET_KEY was not encrypted"
  exit 1
fi

# PUBLIC_URL must NOT be encrypted (outside mask)
if ! grep -q 'PUBLIC_URL="https://example.com"' "${DIR}/.env"; then
  echo "${0} error: PUBLIC_URL was modified or encrypted (should be unchanged)"
  exit 1
fi

# ENCODEMASK line itself must be unchanged (it is in the file as a var but not
# matched by its own regex, so it passes through as-is)
if ! grep -q '^ENCODEMASK=' "${DIR}/.env"; then
  echo "${0} error: ENCODEMASK line is missing"
  exit 1
fi

# Regular single-# comment must pass through unchanged
if ! grep -qF '# regular single-hash comment – must pass through unchanged' "${DIR}/.env"; then
  echo "${0} error: single-# comment was not preserved"
  exit 1
fi

# ---------------------------------------------------------------------------
# test010b — ## comment lines: encrypted variants must carry securenv: prefix
# ---------------------------------------------------------------------------

# "##simple double-hash comment with content" → encrypted
if ! grep -q '^## securenv:' "${DIR}/.env"; then
  echo "${0} error: '## simple double-hash comment' was not encrypted"
  exit 1
fi

# "##no space after hashes" → encrypted, prefix is exactly "##"
if ! grep -q '^##securenv:' "${DIR}/.env"; then
  echo "${0} error: '##no space after hashes' was not encrypted"
  exit 1
fi

# "##  two spaces" → encrypted, prefix is "##  "
if ! grep -q '^##  securenv:' "${DIR}/.env"; then
  echo "${0} error: '##  two spaces' was not encrypted"
  exit 1
fi

# "##<TAB>one tab" → encrypted, prefix is "##<TAB>"
if ! grep -qE $'^##\tsecurenv:' "${DIR}/.env"; then
  echo "${0} error: '##<TAB>one tab' was not encrypted"
  exit 1
fi

# "##<TAB><TAB>two tabs" → encrypted, prefix is "##<TAB><TAB>"
if ! grep -qE $'^##\t\tsecurenv:' "${DIR}/.env"; then
  echo "${0} error: '##<TAB><TAB>two tabs' was not encrypted"
  exit 1
fi

# "##<TAB> tab then space" → prefix is "##<TAB> "
if ! grep -qE $'^##\t securenv:' "${DIR}/.env"; then
  echo "${0} error: '##<TAB> tab then space' was not encrypted"
  exit 1
fi

# "## <TAB>space then tab" → prefix is "## <TAB>"
if ! grep -qE $'^## \tsecurenv:' "${DIR}/.env"; then
  echo "${0} error: '## <TAB>space then tab' was not encrypted"
  exit 1
fi

# "  ## leading two spaces" → prefix is "  ## "
if ! grep -q '^  ## securenv:' "${DIR}/.env"; then
  echo "${0} error: '  ## leading two spaces' was not encrypted"
  exit 1
fi

# "<TAB>## leading tab" → prefix is "<TAB>## "
if ! grep -qE $'^\t## securenv:' "${DIR}/.env"; then
  echo "${0} error: '<TAB>## leading tab' was not encrypted"
  exit 1
fi

# "### triple hash" → extra # is payload, prefix is "##", encrypted payload
# starts with securenv: immediately after "##"
if ! grep -q '^###' "${DIR}/.env"; then
  # after encryption "###..." becomes "##securenv:..." so the raw "###" must be gone
  # but the new line starts with "##securenv:" which already checked above (##no space)
  # Just verify the original "### triple hash" literal is gone
  :
fi
# The encrypted form of "### triple hash..." starts with "##" then "securenv:"
# We already checked "##securenv:" above. Additionally check the plaintext is gone.
if grep -qF '### triple hash' "${DIR}/.env"; then
  echo "${0} error: '### triple hash' payload was not encrypted"
  exit 1
fi

# "#### four hashes" → prefix "##", payload "## four hashes..." encrypted
if grep -qF '#### four hashes' "${DIR}/.env"; then
  echo "${0} error: '#### four hashes' payload was not encrypted"
  exit 1
fi

# ---------------------------------------------------------------------------
# test010c — ignored ## lines (only spaces/tabs after ##) must be unchanged
# ---------------------------------------------------------------------------

# "##    " (spaces only after ##) must remain literally
if ! grep -qE $'^##[[:space:]]+$' "${DIR}/.env"; then
  echo "${0} error: whitespace-only ## lines were not preserved"
  exit 1
fi

# "## " (one space) must remain literally
if ! grep -qF '## ' "${DIR}/.env"; then
  echo "${0} error: '## ' whitespace-only line was not preserved"
  exit 1
fi

# ---------------------------------------------------------------------------
# test010d — decrypt (round-trip)
# ---------------------------------------------------------------------------

CMD_DEC="./bin/${ARCH_DIR}/securenv decrypt test/test010/.env test/test010/.env.dec --mask ENCODEMASK --password \"testpwd010\""
(cd "${ROOT}" && eval "${CMD_DEC}")
STATUS_DEC=$?

if [[ "${STATUS_DEC}" -ne 0 ]]; then
  echo "${0} error: decrypt failed with exit code >${STATUS_DEC}<"
  exit 1
fi

if [[ ! -f "${DIR}/.env.dec" ]]; then
  echo "${0} error: decrypted file >${DIR}/.env.dec< was not created"
  exit 1
fi

# Round-trip: decrypted file must match original .env.tmp exactly
if ! diff -q "${DIR}/.env.tmp" "${DIR}/.env.dec" > /dev/null 2>&1; then
  echo "${0} error: round-trip failed – decrypted output does not match original .env.tmp"
  diff "${DIR}/.env.tmp" "${DIR}/.env.dec" || true
  exit 1
fi

echo "  ${CMD_ENC}"
echo "  ${CMD_DEC}"
exit 0
