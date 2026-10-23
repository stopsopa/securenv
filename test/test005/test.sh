DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd "${DIR}/../.." && pwd)"

source "${ROOT}/bash/detect_arch.sh"
BIN="${SECURENV_BIN}"

if [[ ! -f "${BIN}" ]]; then
  echo "${0} error: binary not found at >${BIN}<"
  exit 1
fi

CMD_ENC="\"${BIN}\" encrypt \"${DIR}/.env.tmp\" \"${DIR}/.env\" --mask ENCODEMASK --password \"complex-p@ssw0rd!#123\""
eval "${CMD_ENC}"
STATUS_ENC=$?

if [[ "${STATUS_ENC}" -ne 0 ]]; then
  echo "${0} error: encrypt failed with exit code >${STATUS_ENC}<"
  exit 1
fi

CMD_DEC="\"${BIN}\" decrypt \"${DIR}/.env\" \"${DIR}/.env.dec\" --mask ENCODEMASK --password \"complex-p@ssw0rd!#123\""
eval "${CMD_DEC}"
STATUS_DEC=$?

if [[ "${STATUS_DEC}" -ne 0 ]]; then
  echo "${0} error: decrypt failed with exit code >${STATUS_DEC}<"
  exit 1
fi

# Compare raw content of .env.tmp and .env.dec directly
ORIG_LINE=$(grep '^API_SECRET=' "${DIR}/.env.tmp")
DEC_LINE=$(grep '^API_SECRET=' "${DIR}/.env.dec")

if [[ "${ORIG_LINE}" != "${DEC_LINE}" ]]; then
  echo "${0} error: decrypted API_SECRET did not match original value"
  echo "Expected: >${ORIG_LINE}<"
  echo "Got:      >${DEC_LINE}<"
  exit 1
fi

ORIG_PRIV=$(grep '^PRIVATE_KEY=' "${DIR}/.env.tmp")
DEC_PRIV=$(grep '^PRIVATE_KEY=' "${DIR}/.env.dec")

if [[ "${ORIG_PRIV}" != "${DEC_PRIV}" ]]; then
  echo "${0} error: decrypted PRIVATE_KEY did not match original value"
  echo "Expected: >${ORIG_PRIV}<"
  echo "Got:      >${DEC_PRIV}<"
  exit 1
fi

if ! grep -F -q 'PUBLIC_NAME="test-app"' "${DIR}/.env.dec"; then
  echo "${0} error: PUBLIC_NAME was modified or missing"
  exit 1
fi

echo "  ${CMD_ENC}"
echo "  ${CMD_DEC}"
exit 0
