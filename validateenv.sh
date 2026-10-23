DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Validation script for local development toolchain readiness.
#
# Checks that the following tools are installed and accessible:
#   - rustc   (Rust compiler)
#   - cargo   (Rust package manager / build tool)
#   - rustfmt (Rust code formatter)
#   - clippy  (Rust linter)
#
# Also runs `cargo fetch` to confirm dependencies resolve correctly.
#
# Exit 0 = toolchain is ready for local development.
# Exit 1 = something is missing or broken.
#
# To install Rust on macOS:
#   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
#
# Then add the arm64 target:
#   rustup target add aarch64-apple-darwin

GREEN=$'\033[32m'
RED=$'\033[31m'
RESET=$'\033[0m'

FAILED=0

check_tool() {
    local tool="${1}"
    local version_flag="${2}"

    if command -v "${tool}" > /dev/null 2>&1; then
        local version
        version=$("${tool}" "${version_flag}" 2>&1 | head -1)
        echo "${GREEN}OK${RESET}  ${tool}: ${version}"
    else
        echo "${RED}MISSING${RESET}  ${tool} not found in PATH"
        FAILED=1
    fi
}

LOG="${DIR}/validateenv.log"

echo ""
echo "Checking Rust toolchain..."
echo ""

{
    check_tool rustc         --version
    check_tool cargo         --version
    check_tool rustfmt       --version
    check_tool clippy-driver --version
} 2>&1 | tee "${LOG}"

echo ""

if [[ "${FAILED}" -eq 1 ]]; then
    cat <<EEE
${RED}
Some tools are missing. Install Rust via:

  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

Then add the arm64 target:

  rustup target add aarch64-apple-darwin
${RESET}
EEE
    exit 1
fi

echo "Running cargo fetch to verify dependencies resolve..."
echo ""

cargo fetch --manifest-path "${DIR}/Cargo.toml"

if [[ $? -ne 0 ]]; then
    echo "${RED}cargo fetch failed - check Cargo.toml and network connectivity${RESET}"
    exit 1
fi

echo ""
echo "${GREEN}Toolchain ready.${RESET}"
