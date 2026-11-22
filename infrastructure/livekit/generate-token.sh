#!/usr/bin/env bash
set -Eeuo pipefail
set +x

readonly MAX_TTL=3600
readonly DEFAULT_TTL=3600

usage() {
  echo "usage: LIVEKIT_API_KEY=... LIVEKIT_API_SECRET=... $0 <room> <identity> [ttl_seconds]" >&2
  echo "ttl_seconds: 1..${MAX_TTL}, default ${DEFAULT_TTL}" >&2
  exit 2
}

fail() {
  echo "error: $1" >&2
  exit 1
}

base64url() {
  openssl base64 -A | tr '+/' '-_' | tr -d '='
}

[ "$#" -ge 2 ] && [ "$#" -le 3 ] || usage

room=$1
identity=$2
ttl=${3:-$DEFAULT_TTL}

[ -n "${LIVEKIT_API_KEY:-}" ] || fail "LIVEKIT_API_KEY is not set"
[ -n "${LIVEKIT_API_SECRET:-}" ] || fail "LIVEKIT_API_SECRET is not set"
[ "${#LIVEKIT_API_SECRET}" -ge 32 ] || fail "LIVEKIT_API_SECRET must have at least 32 characters"

command -v openssl >/dev/null 2>&1 || fail "openssl is required"

name_pattern='^[A-Za-z0-9._:@-]{1,128}$'
[[ $room =~ $name_pattern ]] || fail "room must match ${name_pattern}"
[[ $identity =~ $name_pattern ]] || fail "identity must match ${name_pattern}"
[[ $LIVEKIT_API_KEY =~ $name_pattern ]] || fail "LIVEKIT_API_KEY has unsupported characters"
[[ $ttl =~ ^[0-9]+$ ]] || fail "ttl_seconds must be an integer"
[ "$ttl" -ge 1 ] && [ "$ttl" -le "$MAX_TTL" ] || fail "ttl_seconds must be between 1 and ${MAX_TTL}"

now=$(date -u +%s)
exp=$((now + ttl))

header='{"alg":"HS256","typ":"JWT"}'
payload=$(printf '{"iss":"%s","sub":"%s","name":"%s","nbf":%d,"exp":%d,"video":{"room":"%s","roomJoin":true,"canPublish":true,"canSubscribe":true,"canPublishData":true,"canPublishSources":["microphone","camera"]}}' \
  "$LIVEKIT_API_KEY" "$identity" "$identity" "$now" "$exp" "$room")

signing_input="$(printf '%s' "$header" | base64url).$(printf '%s' "$payload" | base64url)"
signature=$(printf '%s' "$signing_input" | openssl dgst -sha256 -hmac "$LIVEKIT_API_SECRET" -binary | base64url)

printf '%s.%s\n' "$signing_input" "$signature"
