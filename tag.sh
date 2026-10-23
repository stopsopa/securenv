GRAY=$'\033[38;5;244m'
RED=$'\033[31m'
GREEN=$'\033[32m'
YELLOW=$'\033[33m'
CYAN=$'\033[36m'
BOLD=$'\033[1m'
RESET=$'\033[0m'

MODE="${1}"

if [[ "${MODE}" != "patch" && "${MODE}" != "minor" && "${MODE}" != "major" ]]; then
  cat <<EEE
${YELLOW}Usage:${RESET}
  /bin/bash tag.sh patch
  /bin/bash tag.sh minor
  /bin/bash tag.sh major

${CYAN}Options:${RESET}
  patch  - Increment patch version (x.y.Z+1)
  minor  - Increment minor version (x.Y+1.0)
  major  - Increment major version (X+1.0.0)
EEE
  exit 1
fi

echo "${GRAY}Fetching git tags...${RESET}"
git fetch --tags || {
  echo "${0} error: git fetch --tags failed"
  exit 1
}

ALL_TAGS="$(git tag -l)"

LATEST_TAG=""
if [[ -n "${ALL_TAGS}" ]]; then
  LATEST_TAG="$(git tag -l --sort=v:refname | grep -E '^v?[0-9]+\.[0-9]+\.[0-9]+$' | tail -n 1)"
fi

V_PREFIX=""
if [[ -z "${LATEST_TAG}" ]]; then
  LATEST_TAG="0.0.0"
  MAJOR=0
  MINOR=0
  PATCH=0
else
  RAW_TAG="${LATEST_TAG}"
  if [[ "${RAW_TAG}" =~ ^v ]]; then
    V_PREFIX="v"
    RAW_TAG="${RAW_TAG#v}"
  fi
  MAJOR="$(echo "${RAW_TAG}" | cut -d. -f1)"
  MINOR="$(echo "${RAW_TAG}" | cut -d. -f2)"
  PATCH="$(echo "${RAW_TAG}" | cut -d. -f3)"
fi

if [[ "${MODE}" == "patch" ]]; then
  PATCH=$((PATCH + 1))
elif [[ "${MODE}" == "minor" ]]; then
  MINOR=$((MINOR + 1))
  PATCH=0
elif [[ "${MODE}" == "major" ]]; then
  MAJOR=$((MAJOR + 1))
  MINOR=0
  PATCH=0
fi

NEW_TAG="${V_PREFIX}${MAJOR}.${MINOR}.${PATCH}"

cat <<EEE

${BOLD}Previous tag:${RESET} ${YELLOW}${LATEST_TAG}${RESET}
${BOLD}Mode:${RESET}         ${CYAN}${MODE}${RESET}
${BOLD}New tag:${RESET}      ${GREEN}${NEW_TAG}${RESET}

EEE

read -r -p "Press Enter to create and push tag '${NEW_TAG}', or Ctrl+C to abort..."

git tag "${NEW_TAG}" || {
  echo "${0} error: failed to create tag >${NEW_TAG}<"
  exit 1
}

git push origin "${NEW_TAG}" || {
  echo "${0} error: failed to push tag >${NEW_TAG}< to remote"
  exit 1
}

echo "${GREEN}Successfully created and pushed tag >${NEW_TAG}<${RESET}"
