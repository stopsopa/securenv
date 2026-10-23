DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd "${DIR}/../.." && pwd)"

source "${ROOT}/bash/detect_arch.sh"
BIN="${SECURENV_BIN}"

if [[ ! -f "${BIN}" ]]; then
  echo "${0} error: binary not found at >${BIN}<"
  exit 1
fi

CMD="./bin/${ARCH_DIR}/securenv encrypt test/test007/.env.tmp --mask ENCODEMASK --password \"pwd\" --stdout"
OUT=$(cd "${ROOT}" && eval "${CMD}")
STATUS_STDOUT=$?

if [[ "${STATUS_STDOUT}" -ne 0 ]]; then
  echo "${0} error: encrypt with --stdout failed with exit code >${STATUS_STDOUT}<"
  exit 1
fi

if [[ "${OUT}" != *"APP_TOKEN=\"securenv:"* ]]; then
  echo "${0} error: stdout output does not contain expected encrypted APP_TOKEN"
  exit 1
fi

CMD_CONFLICT="./bin/${ARCH_DIR}/securenv encrypt test/test007/.env.tmp test/test007/.env --mask ENCODEMASK --password \"pwd\" --stdout"
ERR_OUT=$(cd "${ROOT}" && eval "${CMD_CONFLICT}" 2>&1)
STATUS_CONFLICT=$?

if [[ "${STATUS_CONFLICT}" -eq 0 ]]; then
  echo "${0} error: expected failure when providing both target and --stdout, got exit code 0"
  exit 1
fi

if [[ "${ERR_OUT}" != *"securenv error:"* ]]; then
  echo "${0} error: missing 'securenv error:' prefix when --stdout conflict occurs"
  exit 1
fi

echo "  ${CMD}"
exit 0
