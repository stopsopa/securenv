DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd "${DIR}/../.." && pwd)"

source "${ROOT}/bash/detect_arch.sh"
BIN="${SECURENV_BIN}"

if [[ ! -f "${BIN}" ]]; then
  echo "${0} error: binary not found at >${BIN}<"
  exit 1
fi

CMD="./bin/${ARCH_DIR}/securenv encrypt test/test002/.env.tmp test/test002/.env --mask ENCODEMASK --password \"pwd\""
ERR_OUT=$(cd "${ROOT}" && eval "${CMD}" 2>&1)
STATUS=$?

if [[ "${STATUS}" -eq 0 ]]; then
  echo "${0} error: expected failure due to malformed regex, but got exit code 0"
  exit 1
fi

if [[ "${ERR_OUT}" != *"securenv error:"* ]]; then
  echo "${0} error: stderr missing 'securenv error:' prefix. Output was >${ERR_OUT}<"
  exit 1
fi

if [[ "${ERR_OUT}" != *"invalid regex in mask var"* ]]; then
  echo "${0} error: stderr missing 'invalid regex in mask var'. Output was >${ERR_OUT}<"
  exit 1
fi

echo "  ${CMD}"
exit 0
