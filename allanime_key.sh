#!/bin/bash
# Derive live AllAnime aaReq crypto constants from mkissa.to.
# Progress on stderr; machine-readable lines on stdout:
#   EPOCH: <n>
#   KEY: <64 hex chars>
#   BUILD_ID: <n>   (from crypto chunk ln="…", used as x-build-id / allanime_build_id)
#
# Used by ani-cli-jq-json.sh (aaReq / fix branches).
set -euo pipefail

UA="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"

echo "→ Fetching HTML..." >&2
HTML=$(curl -fsS -H "User-Agent: $UA" "https://mkissa.to")

echo "→ Extracting crypto data..." >&2
AA_CRYPTO=$(printf '%s' "$HTML" | grep -o 'window\.__aaCrypto\s*=\s*{[^}]*}' | sed 's/window\.__aaCrypto\s*=\s*//' | head -n1)
if [[ -z "$AA_CRYPTO" ]]; then
  echo "error: __aaCrypto blob not found on mkissa.to" >&2
  exit 1
fi

PART_B=$(printf '%s' "$AA_CRYPTO" | grep -o '"partB":"[^"]*"' | cut -d'"' -f4 | head -n1)
EPOCH=$(printf '%s' "$AA_CRYPTO" | grep -o '"epoch":[0-9]*' | cut -d':' -f2 | head -n1)
if [[ -z "$PART_B" || -z "$EPOCH" ]]; then
  echo "error: partB/epoch missing in __aaCrypto" >&2
  exit 1
fi

CDN_IMMUTABLE="https://cdn.allanime.day/all/mk/_app/immutable/"

echo "→ Finding app.js..." >&2
# Prefer entry/app.*.js relative to _app/immutable/ (anipy-cli style).
APP_JS=$(printf '%s' "$HTML" | grep -oE '_app/immutable/(entry/app\.[^"'"'"']+\.js)' | head -n1 | sed 's|^_app/immutable/||')
if [[ -z "$APP_JS" ]]; then
  echo "error: app.js path not found on mkissa.to" >&2
  exit 1
fi

echo "→ Scanning quoted ../chunks for lone mask..." >&2
# anipy-cli keygen: match quoted "../chunks/….js" (SvelteKit client manifest arrays),
# not only ES `import`/`from` (those miss the crypto chunk).
# Accept mask only when the chunk mentions __aaCrypto and contains exactly one 64-hex.
#
# NOTE: do not use `printf | grep -q` under `pipefail` on these megabyte one-line
# bundles — grep -q closes early → SIGPIPE (141) → false negative.
CHUNK=""
CRYPTO_JS=""
while read -r c; do
  [[ -z "$c" ]] && continue
  JS=$(curl -fsS -H "User-Agent: $UA" "${CDN_IMMUTABLE}${c}")
  if [[ "$JS" != *__aaCrypto* ]]; then
    continue
  fi
  mapfile -t masks < <(printf '%s' "$JS" | grep -oE '[a-f0-9]{64}' || true)
  if [[ ${#masks[@]} -eq 1 ]]; then
    CHUNK="${masks[0]}"
    CRYPTO_JS="$JS"
    echo "→ mask chunk: ${c}" >&2
    break
  fi
done < <(
  curl -fsS -H "User-Agent: $UA" "${CDN_IMMUTABLE}${APP_JS}" |
    grep -oE '["'"'"']\.\./chunks/[A-Za-z0-9_-]+\.js["'"'"']' |
    sed -E 's|.*\.\./(chunks/[A-Za-z0-9_-]+\.js).*|\1|' |
    sort -u
)

if [[ -z "$CHUNK" || ${#CHUNK} -ne 64 ]]; then
  echo "error: crypto mask not found (need __aaCrypto chunk with exactly one 64-hex)" >&2
  exit 1
fi

# buildId sits next to the mask in the same chunk: …"MASK":"",ln="64";…
# (also used as bootstrap ?buildId= via encodeURIComponent(ln)).
BUILD_ID="$(printf '%s' "$CRYPTO_JS" | grep -oE "${CHUNK}.{0,30}ln=\"[^\"]+\"" | grep -oE 'ln="[^"]+"' | head -n1 | cut -d'"' -f2 || true)"
if [[ -z "$BUILD_ID" ]]; then
  BUILD_ID="$(printf '%s' "$CRYPTO_JS" | grep -oE 'ln="[0-9]+"' | head -n1 | cut -d'"' -f2 || true)"
fi
if [[ -z "$BUILD_ID" ]]; then
  echo "error: build id (ln=…) not found in crypto chunk" >&2
  exit 1
fi
echo "→ build id: ${BUILD_ID}" >&2

echo "→ Deriving key via XOR..." >&2

# Decode base64 Part B to hex
PART_B_HEX=$(printf '%s' "$PART_B" | base64 -d 2>/dev/null | xxd -p -c 256)
if [[ -z "$PART_B_HEX" || ${#PART_B_HEX} -lt 64 ]]; then
  echo "error: partB base64/hex decode failed" >&2
  exit 1
fi

# XOR the two hex strings (32 bytes)
XOR_RESULT=""
for i in $(seq 0 2 62); do
  MASK_BYTE=${CHUNK:$i:2}
  PART_B_BYTE=${PART_B_HEX:$i:2}
  XOR_BYTE=$((0x$MASK_BYTE ^ 0x$PART_B_BYTE))
  XOR_RESULT="${XOR_RESULT}$(printf '%02x' $XOR_BYTE)"
done

if [[ ${#XOR_RESULT} -ne 64 ]]; then
  echo "error: derived key length invalid (${#XOR_RESULT})" >&2
  exit 1
fi

printf 'EPOCH: %s\n' "$EPOCH"
printf 'KEY: %s\n' "$XOR_RESULT"
printf 'BUILD_ID: %s\n' "$BUILD_ID"