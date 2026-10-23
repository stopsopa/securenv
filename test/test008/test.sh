DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd "${DIR}/../.." && pwd)"

source "${ROOT}/bash/detect_arch.sh"
BIN="${SECURENV_BIN}"

if [[ ! -f "${BIN}" ]]; then
  echo "${0} error: binary not found at >${BIN}<"
  exit 1
fi

CMD_ENC="\"${BIN}\" encrypt \"${DIR}/.env.tmp\" \"${DIR}/.env\" --mask ENCODEMASK --password \"pwd\" --prefix \"mycustomprefix:\""
eval "${CMD_ENC}"
STATUS_ENC=$?

if [[ "${STATUS_ENC}" -ne 0 ]]; then
  echo "${0} error: encrypt with --prefix failed with exit code >${STATUS_ENC}<"
  exit 1
fi

if ! grep -q 'CUSTOM_SECRET="mycustomprefix:' "${DIR}/.env"; then
  echo "${0} error: encrypted value missing custom prefix 'mycustomprefix:'"
  exit 1
fi

CMD_DEC="\"${BIN}\" decrypt \"${DIR}/.env\" \"${DIR}/.env.dec\" --mask ENCODEMASK --password \"pwd\" --prefix \"mycustomprefix:\""
eval "${CMD_DEC}"
STATUS_DEC=$?

if [[ "${STATUS_DEC}" -ne 0 ]]; then
  echo "${0} error: decrypt with --prefix failed with exit code >${STATUS_DEC}<"
  exit 1
fi

if ! grep -q 'CUSTOM_SECRET="custom_prefix_test"' "${DIR}/.env.dec"; then
  echo "${0} error: decrypted CUSTOM_SECRET did not match original value"
  exit 1
fi

echo "  ${CMD_ENC}"
echo "  ${CMD_DEC}"
exit 0
