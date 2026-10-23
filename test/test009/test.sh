DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd "${DIR}/../.." && pwd)"

source "${ROOT}/bash/detect_arch.sh"
BIN="${SECURENV_BIN}"

if [[ ! -f "${BIN}" ]]; then
  echo "${0} error: binary not found at >${BIN}<"
  exit 1
fi

CMD_ENC="./bin/${ARCH_DIR}/securenv encrypt test/test009/.env.tmp test/test009/.env --mask ENCODEMASK --password \"testpwd009\""
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

if ! grep -q 'PUBLIC_KEY="securenv:' "${DIR}/.env"; then
  echo "${0} error: PUBLIC_KEY was not encrypted"
  exit 1
fi

if ! grep -q 'FIREBASE_ID="securenv:' "${DIR}/.env"; then
  echo "${0} error: FIREBASE_ID was not encrypted"
  exit 1
fi

if ! grep -q 'NODE_API_URL="securenv:' "${DIR}/.env"; then
  echo "${0} error: NODE_API_URL was not encrypted"
  exit 1
fi

if ! grep -q 'PROJECT_NAME="securenv:' "${DIR}/.env"; then
  echo "${0} error: PROJECT_NAME was not encrypted"
  exit 1
fi

if ! grep -q 'PROJECT_NAME_EXTRA="should_not_be_encrypted"' "${DIR}/.env"; then
  echo "${0} error: PROJECT_NAME_EXTRA was modified or encrypted (anchor failed)"
  exit 1
fi

if ! grep -q 'OTHER_VAR="plain_value"' "${DIR}/.env"; then
  echo "${0} error: OTHER_VAR was modified or encrypted"
  exit 1
fi

CMD_DEC="./bin/${ARCH_DIR}/securenv decrypt test/test009/.env test/test009/.env.dec --mask ENCODEMASK --password \"testpwd009\""
(cd "${ROOT}" && eval "${CMD_DEC}")
STATUS_DEC=$?

if [[ "${STATUS_DEC}" -ne 0 ]]; then
  echo "${0} error: decrypt failed with exit code >${STATUS_DEC}<"
  exit 1
fi

if ! grep -q 'PROJECT_NAME="my_project"' "${DIR}/.env.dec"; then
  echo "${0} error: decrypted PROJECT_NAME did not match original value"
  exit 1
fi

echo "  ${CMD_ENC}"
echo "  ${CMD_DEC}"
exit 0
