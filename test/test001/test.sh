DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd "${DIR}/../.." && pwd)"

source "${ROOT}/bash/detect_arch.sh"
BIN="${SECURENV_BIN}"

if [[ ! -f "${BIN}" ]]; then
  echo "${0} error: binary not found at >${BIN}<"
  exit 1
fi

CMD="\"${BIN}\" encrypt \"${DIR}/.env.tmp\" \"${DIR}/.env\" --mask ENCODEMASK --password \"testpwd001\""
eval "${CMD}"
STATUS=$?

if [[ "${STATUS}" -ne 0 ]]; then
  echo "${0} error: encrypt failed with exit code >${STATUS}<"
  exit 1
fi

if [[ ! -f "${DIR}/.env" ]]; then
  echo "${0} error: output file >${DIR}/.env< was not created"
  exit 1
fi

if ! grep -q "# Database configuration" "${DIR}/.env"; then
  echo "${0} error: comment line '# Database configuration' was lost"
  exit 1
fi

if ! grep -q 'DB_HOST="localhost"' "${DIR}/.env"; then
  echo "${0} error: DB_HOST was modified or missing"
  exit 1
fi

if ! grep -q 'DB_PORT=5432' "${DIR}/.env"; then
  echo "${0} error: DB_PORT was modified or missing"
  exit 1
fi

if ! grep -q 'SECRET_KEY="securenv:' "${DIR}/.env"; then
  echo "${0} error: SECRET_KEY was not encrypted with securenv: prefix"
  exit 1
fi

if ! grep -q 'SECRET_API_TOKEN="securenv:' "${DIR}/.env"; then
  echo "${0} error: SECRET_API_TOKEN was not encrypted with securenv: prefix"
  exit 1
fi

if ! grep -q 'APP_ENV=development' "${DIR}/.env"; then
  echo "${0} error: APP_ENV was modified or missing"
  exit 1
fi

echo "  ${CMD}"
exit 0
