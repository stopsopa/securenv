DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd "${DIR}/../.." && pwd)"

source "${ROOT}/bash/detect_arch.sh"
BIN="${SECURENV_BIN}"

if [[ ! -f "${BIN}" ]]; then
  echo "${0} error: binary not found at >${BIN}<"
  exit 1
fi

CMD_ENC="./bin/${ARCH_DIR}/securenv encrypt test/test012/.env.tmp test/test012/.env --password \"testpwd012\""
(cd "${ROOT}" && eval "${CMD_ENC}")
STATUS_ENC=$?

if [[ "${STATUS_ENC}" -ne 0 ]]; then
  echo "${0} error: encrypt failed with exit code >${STATUS_ENC}<"
  exit 1
fi

# untouched file must decrypt back to original
CMD_DEC="./bin/${ARCH_DIR}/securenv decrypt test/test012/.env test/test012/.env.dec --password \"testpwd012\""
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

# swap encrypted values between SECRET_A and SECRET_B
VALUE_A="$(grep '^SECRET_A=' "${DIR}/.env" | cut -d= -f2-)"
VALUE_B="$(grep '^SECRET_B=' "${DIR}/.env" | cut -d= -f2-)"
printf 'SECRET_A=%s\nSECRET_B=%s\n' "${VALUE_B}" "${VALUE_A}" > "${DIR}/.env.swapped"

CMD_SWAPPED="./bin/${ARCH_DIR}/securenv decrypt test/test012/.env.swapped --stdout --password \"testpwd012\""
ERR_OUT=$(cd "${ROOT}" && eval "${CMD_SWAPPED}" 2>&1)
STATUS_SWAPPED=$?

if [[ "${STATUS_SWAPPED}" -eq 0 ]]; then
  echo "${0} error: expected decrypt failure after swapping values between vars, but got exit code 0"
  exit 1
fi

if [[ "${ERR_OUT}" != *"securenv error: decrypt failed for var 'SECRET_A'"* ]]; then
  echo "${0} error: stderr missing \"securenv error: decrypt failed for var 'SECRET_A'\". Output was >${ERR_OUT}<"
  exit 1
fi

echo "  ${CMD_ENC}"
echo "  ${CMD_DEC}"
echo "  ${CMD_SWAPPED}"
exit 0
