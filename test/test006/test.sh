DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd "${DIR}/../.." && pwd)"

source "${ROOT}/bash/detect_arch.sh"
BIN="${SECURENV_BIN}"

if [[ ! -f "${BIN}" ]]; then
  echo "${0} error: binary not found at >${BIN}<"
  exit 1
fi

# 1. Missing password env var
CMD_MISSING="./bin/${ARCH_DIR}/securenv encrypt test/test006/.env.tmp test/test006/.env --mask ENCODEMASK --password-env \"NON_EXISTENT_VAR_12345\""
ERR_OUT=$(cd "${ROOT}" && eval "${CMD_MISSING}" 2>&1)
STATUS_MISSING=$?
if [[ "${STATUS_MISSING}" -eq 0 ]]; then
  echo "${0} error: expected failure on missing password env var, got exit code 0"
  exit 1
fi
if [[ "${ERR_OUT}" != *"securenv error:"* ]]; then
  echo "${0} error: missing 'securenv error:' prefix when password env var is missing"
  exit 1
fi

# 2. Empty password env var
export EMPTY_TEST_PWD=""
CMD_EMPTY="./bin/${ARCH_DIR}/securenv encrypt test/test006/.env.tmp test/test006/.env --mask ENCODEMASK --password-env \"EMPTY_TEST_PWD\""
ERR_OUT_EMPTY=$(cd "${ROOT}" && eval "${CMD_EMPTY}" 2>&1)
STATUS_EMPTY=$?
if [[ "${STATUS_EMPTY}" -eq 0 ]]; then
  echo "${0} error: expected failure on empty password env var, got exit code 0"
  exit 1
fi
if [[ "${ERR_OUT_EMPTY}" != *"securenv error:"* ]]; then
  echo "${0} error: missing 'securenv error:' prefix when password env var is empty"
  exit 1
fi

# 3. Valid password env var
export VALID_TEST_PWD="my_env_secret_password"
CMD_VALID_ENC="./bin/${ARCH_DIR}/securenv encrypt test/test006/.env.tmp test/test006/.env --mask ENCODEMASK --password-env \"VALID_TEST_PWD\""
(cd "${ROOT}" && eval "${CMD_VALID_ENC}")
STATUS_VALID_ENC=$?
if [[ "${STATUS_VALID_ENC}" -ne 0 ]]; then
  echo "${0} error: encrypt with --password-env failed with exit code >${STATUS_VALID_ENC}<"
  exit 1
fi

CMD_VALID_DEC="./bin/${ARCH_DIR}/securenv decrypt test/test006/.env test/test006/.env.dec --mask ENCODEMASK --password-env \"VALID_TEST_PWD\""
(cd "${ROOT}" && eval "${CMD_VALID_DEC}")
STATUS_VALID_DEC=$?
if [[ "${STATUS_VALID_DEC}" -ne 0 ]]; then
  echo "${0} error: decrypt with --password-env failed with exit code >${STATUS_VALID_DEC}<"
  exit 1
fi

if ! grep -q 'SECRET_DATA="password_env_secret"' "${DIR}/.env.dec"; then
  echo "${0} error: decrypted SECRET_DATA did not match original value"
  exit 1
fi

echo "  export VALID_TEST_PWD=\"my_env_secret_password\""
echo "  ${CMD_VALID_ENC}"
echo "  ${CMD_VALID_DEC}"
exit 0
