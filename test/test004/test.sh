DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd "${DIR}/../.." && pwd)"

source "${ROOT}/bash/detect_arch.sh"
BIN="${SECURENV_BIN}"

if [[ ! -f "${BIN}" ]]; then
  echo "${0} error: binary not found at >${BIN}<"
  exit 1
fi

CMD="./bin/${ARCH_DIR}/securenv encrypt test/test004/.env.tmp test/test004/nonexistent_folder/.env --password \"pwd\""
ERR_OUT=$(cd "${ROOT}" && eval "${CMD}" 2>&1)
STATUS=$?

if [[ "${STATUS}" -eq 0 ]]; then
  echo "${0} error: expected failure due to nonexistent target parent directory, but got exit code 0"
  exit 1
fi

if [[ "${ERR_OUT}" != *"securenv error:"* ]]; then
  echo "${0} error: stderr missing 'securenv error:' prefix. Output was >${ERR_OUT}<"
  exit 1
fi

if [[ "${ERR_OUT}" != *"parent directory"* ]]; then
  echo "${0} error: stderr missing 'parent directory'. Output was >${ERR_OUT}<"
  exit 1
fi

echo "  ${CMD}"
exit 0
