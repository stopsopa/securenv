DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

GREEN=$'\033[32m'
RESET=$'\033[0m'

TARGET="aarch64-apple-darwin"

echo "Building for ${TARGET}..."

cargo build --release --target "${TARGET}" --manifest-path "${DIR}/Cargo.toml"

mkdir -p "${DIR}/bin"

cp "${DIR}/target/${TARGET}/release/securenv" "${DIR}/bin/arm64"

echo "${GREEN}Done. Binary written to bin/arm64${RESET}"
