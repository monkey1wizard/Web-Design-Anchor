#!/usr/bin/env bash
#
# publish-public.sh - curated one-way export of the private source of truth
# to the public GitHub remote.
#
# Usage:
#   packaging/publish-public.sh
#   packaging/publish-public.sh --keep-snapshot <dir>
#   packaging/publish-public.sh --execute
#
# The default is a dry run. Only --execute pushes the curated snapshot.

set -euo pipefail

REMOTE="github"
BRANCH="main"
EXECUTE=0
KEEP_SNAPSHOT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --execute) EXECUTE=1 ;;
    --keep-snapshot)
      KEEP_SNAPSHOT="${2:?--keep-snapshot needs a value}"
      shift
      ;;
    --remote)
      REMOTE="${2:?--remote needs a value}"
      shift
      ;;
    --branch)
      BRANCH="${2:?--branch needs a value}"
      shift
      ;;
    -h|--help)
      sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "publish-public: unknown argument '$1'" >&2
      exit 2
      ;;
  esac
  shift
done

if [[ "$EXECUTE" -eq 1 && -n "$KEEP_SNAPSHOT" ]]; then
  echo "publish-public: --keep-snapshot is only available for dry runs." >&2
  exit 2
fi

EXCLUDE_TRACKED=(
  ".dev"
  "prefile_*.md"
  "ROADMAP.md"
  "AGENTS.md"
  "CLAUDE.md"
  ".claude"
  ".github/instructions"
)

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"
SOURCE_SHA="$(git rev-parse --short HEAD)"
STAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "publish-public: working tree is dirty - commit or stash first." >&2
  exit 1
fi

if ! PUBLIC_URL="$(git remote get-url "$REMOTE" 2>/dev/null)"; then
  echo "publish-public: public remote '$REMOTE' not found." >&2
  exit 1
fi

if printf '%s' "$PUBLIC_URL" | grep -qi 'gitlab\.com'; then
  echo "publish-public: remote '$REMOTE' points at GitLab ($PUBLIC_URL)." >&2
  echo "  That is the private source of truth - refusing to publish there." >&2
  exit 1
fi
if ! printf '%s' "$PUBLIC_URL" | grep -qi 'github\.com'; then
  echo "publish-public: remote '$REMOTE' ($PUBLIC_URL) is not a github.com URL - refusing." >&2
  exit 1
fi

SNAP="$(mktemp -d)"
cleanup() { rm -rf "$SNAP"; }
trap cleanup EXIT

# git archive provides the committed tree without rewriting any snapshot file.
git archive --format=tar HEAD | tar -x -C "$SNAP"

for path in "${EXCLUDE_TRACKED[@]}"; do
  if [[ "$path" == "prefile_*.md" ]]; then
    find "$SNAP" -maxdepth 1 -type f -name 'prefile_*.md' -delete
  elif [[ -e "$SNAP/$path" || -d "$SNAP/$path" ]]; then
    rm -rf -- "${SNAP:?}/$path"
  fi
done

# Capture private-pattern hits before deletion so the residual tripwire remains live.
PRIVATE_HITS="$(find "$SNAP" -type f -name '*.private.md' -print)"
find "$SNAP" -type f \( -name '*.private.md' -o -name '*.local.*' -o -name 'config.json' \) -delete

PRIVATE_NAME="horiza""creations"
LEAKS=0
for path in "${EXCLUDE_TRACKED[@]}"; do
  if [[ -e "$SNAP/$path" || -d "$SNAP/$path" ]]; then
    echo "LEAK: excluded path remains in snapshot: $path" >&2
    LEAKS=1
  fi
done
if [[ -n "$PRIVATE_HITS" ]]; then
  echo "LEAK: private notes present:" >&2
  echo "$PRIVATE_HITS" >&2
  LEAKS=1
fi
if grep -rIlF -- "$PRIVATE_NAME" "$SNAP" >/dev/null 2>&1; then
  echo "LEAK: private session name detected in snapshot:" >&2
  grep -rIlF -- "$PRIVATE_NAME" "$SNAP" >&2 || true
  LEAKS=1
fi
if grep -rIlE 'BEGIN (RSA |OPENSSH |EC )?PRIVATE KEY|glpat-[A-Za-z0-9_-]{20}|gho_[A-Za-z0-9]{36}|xox[baprs]-[A-Za-z0-9-]+' "$SNAP" >/dev/null 2>&1; then
  echo "LEAK: possible secret material detected in snapshot:" >&2
  grep -rIlE 'BEGIN (RSA |OPENSSH |EC )?PRIVATE KEY|glpat-[A-Za-z0-9_-]{20}|gho_[A-Za-z0-9]{36}|xox[baprs]-[A-Za-z0-9-]+' "$SNAP" >&2 || true
  LEAKS=1
fi
if [[ "$LEAKS" -ne 0 ]]; then
  echo "publish-public: ABORT - residual private content in snapshot. Nothing pushed." >&2
  exit 1
fi

SNAP_FILES="$(find "$SNAP" -type f | wc -l | tr -d ' ')"
git -C "$SNAP" init -q -b "$BRANCH"
git -C "$SNAP" -c user.name='WDA Publish' -c user.email='noreply@web-design-anchor' add -A
git -C "$SNAP" -c user.name='WDA Publish' -c user.email='noreply@web-design-anchor' \
  commit -q -m "Public snapshot from ${SOURCE_SHA} (${STAMP})" \
  -m "Curated export - private planning paths stripped."
git -C "$SNAP" remote add public "$PUBLIC_URL"

if [[ -n "$KEEP_SNAPSHOT" ]]; then
  mkdir -p "$KEEP_SNAPSHOT"
  rm -rf "$KEEP_SNAPSHOT"/* "$KEEP_SNAPSHOT"/.[!.]* "$KEEP_SNAPSHOT"/..?* 2>/dev/null || true
  cp -a "$SNAP"/. "$KEEP_SNAPSHOT"/
fi

echo "---------------------------------------------"
echo "publish-public - curated snapshot"
echo "  source HEAD     : ${SOURCE_SHA}"
echo "  public remote   : ${REMOTE} -> ${PUBLIC_URL}"
echo "  public branch   : ${BRANCH}"
echo "  stripped        : ${EXCLUDE_TRACKED[*]}"
echo "  files published : ${SNAP_FILES}"
echo "  residual scan   : PASS (no excluded paths, private notes, session name, or secret residue)"
echo "---------------------------------------------"

if [[ "$EXECUTE" -ne 1 ]]; then
  echo "DRY RUN - nothing pushed. Re-run without --keep-snapshot and with --execute to force-push the snapshot."
  exit 0
fi

echo "Force-pushing curated snapshot to ${REMOTE}/${BRANCH} ..."
git -C "$SNAP" push --force public "${BRANCH}:${BRANCH}"
echo "Done. Public ${REMOTE}/${BRANCH} now holds the curated snapshot of ${SOURCE_SHA}."
