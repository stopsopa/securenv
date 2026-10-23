_DETECT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
_ROOT_DIR="$(cd "${_DETECT_DIR}/.." && pwd)"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
  Darwin)
    case "${ARCH}" in
      arm64|aarch64)
        ARCH_DIR="arm64"
        TARGET="aarch64-apple-darwin"
        ;;
      x86_64|amd64)
        ARCH_DIR="x64"
        TARGET="x86_64-apple-darwin"
        ;;
      *)
        ARCH_DIR="${ARCH}"
        TARGET=""
        ;;
    esac
    ;;
  Linux)
    case "${ARCH}" in
      arm64|aarch64)
        ARCH_DIR="arm64"
        TARGET="aarch64-unknown-linux-gnu"
        ;;
      x86_64|amd64)
        ARCH_DIR="x64"
        TARGET="x86_64-unknown-linux-gnu"
        ;;
      *)
        ARCH_DIR="${ARCH}"
        TARGET=""
        ;;
    esac
    ;;
  *)
    ARCH_DIR="${ARCH}"
    TARGET=""
    ;;
esac

SECURENV_BIN="${_ROOT_DIR}/bin/${ARCH_DIR}/securenv"
