DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd "${DIR}/../.." && pwd)"

source "${ROOT}/bash/detect_arch.sh"
BIN="${SECURENV_BIN}"

if [[ ! -f "${BIN}" ]]; then
  echo "${0} error: binary not found at >${BIN}<"
  exit 1
fi

# ---------------------------------------------------------------------------
# encrypt
# ---------------------------------------------------------------------------

CMD_ENC="./bin/${ARCH_DIR}/securenv encrypt test/test011/.env.tmp test/test011/.env --mask ENCODEMASK --password \"testpwd011\""
(cd "${ROOT}" && eval "${CMD_ENC}")
STATUS_ENC=$?

if [[ "${STATUS_ENC}" -ne 0 ]]; then
  echo "${0} error: encrypt failed with exit code >${STATUS_ENC}<"
  exit 1
fi

# no part of any secret can stay in plaintext
for PLAIN in 'pa$$word' 'inline comment' 'ab"c' 'exported secret' 'spaced' 'BEGIN PRIVATE KEY' 'MIGHAgEAMBMGByqGSM49AgEG' 'END PRIVATE KEY'; do
  if grep -qF "${PLAIN}" "${DIR}/.env"; then
    echo "${0} error: plaintext >${PLAIN}< leaked to encrypted file"
    exit 1
  fi
done

# `export ` prefix must stay and var must be matched by mask
if ! grep -q '^export SECRET_EXPORTED="securenv:' "${DIR}/.env"; then
  echo "${0} error: 'export SECRET_EXPORTED' was not encrypted"
  exit 1
fi

# indentation and spaces around = must stay
if ! grep -q '^  SECRET_INDENTED ="securenv:' "${DIR}/.env"; then
  echo "${0} error: SECRET_INDENTED was not encrypted or lost its indentation"
  exit 1
fi

# multi-line var outside mask must be copied unchanged
if ! grep -qF 'line two"' "${DIR}/.env"; then
  echo "${0} error: PUBLIC_MULTI was modified"
  exit 1
fi

# ---------------------------------------------------------------------------
# decrypt (round-trip)
# ---------------------------------------------------------------------------

CMD_DEC="./bin/${ARCH_DIR}/securenv decrypt test/test011/.env test/test011/.env.dec --mask ENCODEMASK --password \"testpwd011\""
(cd "${ROOT}" && eval "${CMD_DEC}")
STATUS_DEC=$?

if [[ "${STATUS_DEC}" -ne 0 ]]; then
  echo "${0} error: decrypt failed with exit code >${STATUS_DEC}<"
  exit 1
fi

if ! diff "${DIR}/.env.tmp" "${DIR}/.env.dec"; then
  echo "${0} error: round-trip failed – decrypted output does not match original .env.tmp"
  exit 1
fi

# decrypted file holds plaintext secrets – must be readable by owner only
PERMS="$(ls -l "${DIR}/.env.dec" | cut -c1-10)"
if [[ "${PERMS}" != "-rw-------" ]]; then
  echo "${0} error: decrypted file permissions PERMS=>${PERMS}< expected >-rw-------<"
  exit 1
fi

# ---------------------------------------------------------------------------
# never-closed quote must fail
# ---------------------------------------------------------------------------

printf 'SECRET_X="never closed\nOTHER=1\n' > "${DIR}/.env.broken"

CMD_BROKEN="./bin/${ARCH_DIR}/securenv encrypt test/test011/.env.broken --stdout --password \"testpwd011\""
ERR_OUT=$(cd "${ROOT}" && eval "${CMD_BROKEN}" 2>&1)
STATUS_BROKEN=$?

if [[ "${STATUS_BROKEN}" -eq 0 ]]; then
  echo "${0} error: expected failure due to unterminated quote, but got exit code 0"
  exit 1
fi

if [[ "${ERR_OUT}" != *"securenv error:"*"unterminated"* ]]; then
  echo "${0} error: stderr missing 'securenv error: unterminated'. Output was >${ERR_OUT}<"
  exit 1
fi

echo "  ${CMD_ENC}"
echo "  ${CMD_DEC}"
echo "  ${CMD_BROKEN}"
exit 0
