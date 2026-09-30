#!/usr/bin/env bash
set -euo pipefail

die() {
  printf 'preview deployment error: %s\n' "$*" >&2
  exit 1
}

require() {
  [[ -n "${!1:-}" ]] || die "$1 is required"
}

ensure_preview_dir() {
  case "${PREVIEW_DIR}" in
    "${PREVIEW_ROOT}"/*) ;;
    *) die "PREVIEW_DIR must be a child of PREVIEW_ROOT" ;;
  esac
}

compose() {
  docker compose --project-name "${PREVIEW_PROJECT}" \
    --project-directory "${PREVIEW_DIR}" \
    --env-file "${PREVIEW_DIR}/.env" \
    --file "${PREVIEW_DIR}/compose.yml" "$@"
}

ensure_proxy() {
  local infra_dir
  local proxy_compose

  require PREVIEW_ROOT
  require PREVIEW_REPOSITORY_DIR

  infra_dir="${PREVIEW_INFRA_DIR:-$(dirname "${PREVIEW_ROOT}")/infra}"
  proxy_compose="${infra_dir}/traefik-compose.yml"

  install -d -m 750 "${infra_dir}"
  install -m 600 "${PREVIEW_REPOSITORY_DIR}/deploy/proxy/compose.yml" "${proxy_compose}"

  docker compose --project-name angui-proxy \
    --file "${proxy_compose}" \
    up --detach --remove-orphans
}

ensure_runtime_image() {
  if ! docker image inspect "${PREVIEW_RUNTIME_IMAGE}" >/dev/null 2>&1; then
    docker build \
      --file "${PREVIEW_REPOSITORY_DIR}/deploy/preview/Dockerfile.runtime" \
      --tag "${PREVIEW_RUNTIME_IMAGE}" \
      "${PREVIEW_REPOSITORY_DIR}/deploy/preview"
  fi
}

deploy() {
  require PREVIEW_ROOT
  require PREVIEW_ID
  require PREVIEW_PROJECT
  require PREVIEW_HOST
  require PREVIEW_ROUTER
  require PREVIEW_BASE_DOMAIN
  require PREVIEW_ORIGIN
  require PREVIEW_DEMO_PASSWORD
  require PREVIEW_RUNTIME_IMAGE
  require PREVIEW_REPOSITORY_DIR
  require PREVIEW_BACKEND_DIR
  require PREVIEW_FRONTEND_DIR

  # coturn must advertise a numeric relay address. Resolve the public media
  # hostname when an explicit origin IP is not supplied by the deployment.
  if [[ -z "${PREVIEW_PUBLIC_IP:-}" ]]; then
    if ! resolved_ipv4_addresses="$(getent ahostsv4 "livekit-${PREVIEW_HOST}" | awk '{ print $1 }' | sort -u)" \
      || [[ -z "${resolved_ipv4_addresses}" ]] \
      || [[ "${resolved_ipv4_addresses}" == *$'\n'* ]]; then
      die "livekit-${PREVIEW_HOST} must resolve to one public origin IPv4 address, or PREVIEW_PUBLIC_IP must be set"
    fi
    PREVIEW_PUBLIC_IP="${resolved_ipv4_addresses}"
  fi
  if [[ ! "${PREVIEW_PUBLIC_IP}" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
    die "PREVIEW_PUBLIC_IP must be one IPv4 address"
  fi
  local -a ip_octets
  local octet
  IFS=. read -r -a ip_octets <<< "${PREVIEW_PUBLIC_IP}"
  for octet in "${ip_octets[@]}"; do
    if [[ "${octet}" != 0 && "${octet}" == 0* ]] || (( 10#${octet} > 255 )); then
      die "PREVIEW_PUBLIC_IP must be one IPv4 address"
    fi
  done

  if [[ "${AMAP_JSAPI_SECURITY_CODE:-}" == *$'\r'* || "${AMAP_JSAPI_SECURITY_CODE:-}" == *$'\n'* ]]; then
    die "AMAP_JSAPI_SECURITY_CODE must be a single-line secret"
  fi

  # The policy itself contains no endpoint or credential, but a configured
  # transport without a policy would otherwise silently become the disabled
  # "[]" configuration below. Flatten formatted JSON before handing it to
  # Compose and fail before replacing a preview with rule-only AI.
  ai_providers_json="$(printf '%s' "${ANGUI_AI_PROVIDERS_JSON:-}" | tr -d '\r\n')"
  ai_policy_without_whitespace="$(printf '%s' "${ai_providers_json}" | tr -d '[:space:]')"
  if { [[ -z "${ai_policy_without_whitespace}" ]] || [[ "${ai_policy_without_whitespace}" == "[]" ]]; } \
    && { [[ -n "${ANGUI_PREVIEW_AI_ENDPOINT:-}" ]] || [[ -n "${ANGUI_PREVIEW_AI_KEY:-}" ]]; }; then
    die "ANGUI_AI_PROVIDERS_JSON is required when ANGUI_PREVIEW_AI_ENDPOINT or ANGUI_PREVIEW_AI_KEY is set"
  fi

  # Keep AI values out of the Compose dotenv file. A dotenv file is line based,
  # so even a valid formatted JSON policy can be split into unrelated entries
  # before Compose resolves the api environment. Process environment values take
  # precedence over --env-file values during Compose interpolation.
  export ANGUI_AI_PROVIDERS_JSON="${ai_providers_json:-[]}"
  export ANGUI_PREVIEW_AI_ENDPOINT="${ANGUI_PREVIEW_AI_ENDPOINT:-}"
  export ANGUI_PREVIEW_AI_KEY="${ANGUI_PREVIEW_AI_KEY:-}"
  export PREVIEW_ASR_IMAGE="${PREVIEW_ASR_IMAGE:-angui-asr:cpu}"

  # Allocate nonoverlapping media ports across previews on the same host.
  # Keep the allocation lock until the new .env has been installed.
  install -d -m 750 "${PREVIEW_ROOT}"
  exec 9>"${PREVIEW_ROOT}/.voice-ports.lock"
  flock -x 9
  PREVIEW_DIR="${PREVIEW_ROOT}/${PREVIEW_ID}"
  ensure_preview_dir
  preview_slot=""
  if [[ -f "${PREVIEW_DIR}/.env" ]]; then
    preview_slot="$(sed -n 's/^PREVIEW_VOICE_SLOT=//p' "${PREVIEW_DIR}/.env" | head -1)"
  fi
  if [[ ! "${preview_slot}" =~ ^[0-9]+$ ]] || (( preview_slot >= 100 )); then
    used_slots=" "
    for preview_env in "${PREVIEW_ROOT}"/*/.env; do
      [[ -f "${preview_env}" ]] || continue
      existing_slot="$(sed -n 's/^PREVIEW_VOICE_SLOT=//p' "${preview_env}" | head -1)"
      [[ "${existing_slot}" =~ ^[0-9]+$ ]] && used_slots+="${existing_slot} "
    done
    for candidate_slot in $(seq 0 99); do
      if [[ "${used_slots}" != *" ${candidate_slot} "* ]]; then
        preview_slot="${candidate_slot}"
        break
      fi
    done
    [[ -n "${preview_slot}" ]] || die "no free preview voice port blocks remain"
  fi
  PREVIEW_LIVEKIT_UDP_PORT="$((21000 + preview_slot))"
  PREVIEW_LIVEKIT_TCP_PORT="$((22000 + preview_slot))"
  PREVIEW_TURN_PORT="$((23000 + preview_slot))"
  PREVIEW_TURN_RELAY_MIN="$((24000 + preview_slot * 20))"
  PREVIEW_TURN_RELAY_MAX="$((PREVIEW_TURN_RELAY_MIN + 19))"
  PREVIEW_LIVEKIT_URL="${PREVIEW_SCHEME:-https}://livekit-${PREVIEW_HOST}"
  PREVIEW_LIVEKIT_URL="${PREVIEW_LIVEKIT_URL/https:/wss:}"
  PREVIEW_LIVEKIT_URL="${PREVIEW_LIVEKIT_URL/http:/ws:}"
  PREVIEW_TURN_URL="turn:livekit-${PREVIEW_HOST}:${PREVIEW_TURN_PORT}?transport=udp"
  LIVEKIT_API_KEY="angui-${PREVIEW_ID}"
  LIVEKIT_API_SECRET="$(openssl rand -hex 32)"
  TURN_SECRET="$(openssl rand -hex 32)"
  ANGUI_ASR_KEY="$(openssl rand -hex 32)"

  [[ -f "${PREVIEW_BACKEND_DIR}/angui" ]] || die "backend artifact angui is missing"
  [[ -f "${PREVIEW_BACKEND_DIR}/angui-admin" ]] || die "backend artifact angui-admin is missing"
  [[ -f "${PREVIEW_BACKEND_DIR}/migration" ]] || die "backend artifact migration is missing"
  [[ -d "${PREVIEW_FRONTEND_DIR}" ]] || die "frontend artifact directory is missing"

  # GitHub artifact downloads do not reliably preserve executable bits. Restore
  # them before running the configuration-only validation command; install below
  # also applies the final mode to the files mounted into the containers.
  chmod 755 \
    "${PREVIEW_BACKEND_DIR}/angui" \
    "${PREVIEW_BACKEND_DIR}/angui-admin" \
    "${PREVIEW_BACKEND_DIR}/migration"

  if ! "${PREVIEW_BACKEND_DIR}/angui" validate-ai-config; then
    die "ANGUI_AI_PROVIDERS_JSON failed application configuration validation"
  fi

  ensure_runtime_image
  if ! docker image inspect "${PREVIEW_ASR_IMAGE}" >/dev/null 2>&1; then
    docker build --tag "${PREVIEW_ASR_IMAGE}" "${PREVIEW_REPOSITORY_DIR}/deploy/asr"
  fi
  docker volume create angui-preview-asr-models >/dev/null
  ensure_proxy
  install -d -m 700 "${PREVIEW_DIR}/runtime"
  install -m 755 "${PREVIEW_BACKEND_DIR}/angui" "${PREVIEW_DIR}/runtime/angui"
  install -m 755 "${PREVIEW_BACKEND_DIR}/angui-admin" "${PREVIEW_DIR}/runtime/angui-admin"
  install -m 755 "${PREVIEW_BACKEND_DIR}/migration" "${PREVIEW_DIR}/runtime/migration"
  rm -rf -- "${PREVIEW_DIR}/frontend"
  install -d -m 755 "${PREVIEW_DIR}/frontend"
  cp -a "${PREVIEW_FRONTEND_DIR}/." "${PREVIEW_DIR}/frontend/"
  install -m 600 "${PREVIEW_REPOSITORY_DIR}/deploy/preview/compose.local.yml" "${PREVIEW_DIR}/compose.yml"
  install -m 644 "${PREVIEW_REPOSITORY_DIR}/deploy/preview/nginx.conf" "${PREVIEW_DIR}/nginx.conf"
  sed -e "s/tcp_port: 7881/tcp_port: ${PREVIEW_LIVEKIT_TCP_PORT}/" \
      -e "s/udp_port: 7882/udp_port: ${PREVIEW_LIVEKIT_UDP_PORT}/" \
      "${PREVIEW_REPOSITORY_DIR}/deploy/preview/livekit.yaml" > "${PREVIEW_DIR}/livekit.yaml"

  umask 077
  cat > "${PREVIEW_DIR}/.env" <<EOF
PREVIEW_RUNTIME_IMAGE=${PREVIEW_RUNTIME_IMAGE}
PREVIEW_HOST=${PREVIEW_HOST}
PREVIEW_ROUTER=${PREVIEW_ROUTER}
PREVIEW_BASE_DOMAIN=${PREVIEW_BASE_DOMAIN}
PREVIEW_ORIGIN=${PREVIEW_ORIGIN}
PREVIEW_DEMO_PASSWORD=${PREVIEW_DEMO_PASSWORD}
PREVIEW_PROXY_NETWORK=${PREVIEW_PROXY_NETWORK:-angui-proxy}
AMAP_WEBSERVICE_KEY=${AMAP_WEBSERVICE_KEY:-}
AMAP_JSAPI_SECURITY_CODE=${AMAP_JSAPI_SECURITY_CODE:-}
ANGUI_COLLABORATION_LOCATION_RETENTION_HOURS=${ANGUI_COLLABORATION_LOCATION_RETENTION_HOURS:-24}
PREVIEW_SCHEME=${PREVIEW_SCHEME:-https}
PREVIEW_VOICE_SLOT=${preview_slot}
PREVIEW_PUBLIC_IP=${PREVIEW_PUBLIC_IP}
PREVIEW_ASR_IMAGE=${PREVIEW_ASR_IMAGE}
PREVIEW_LIVEKIT_UDP_PORT=${PREVIEW_LIVEKIT_UDP_PORT}
PREVIEW_LIVEKIT_TCP_PORT=${PREVIEW_LIVEKIT_TCP_PORT}
PREVIEW_TURN_PORT=${PREVIEW_TURN_PORT}
PREVIEW_TURN_RELAY_MIN=${PREVIEW_TURN_RELAY_MIN}
PREVIEW_TURN_RELAY_MAX=${PREVIEW_TURN_RELAY_MAX}
PREVIEW_LIVEKIT_URL=${PREVIEW_LIVEKIT_URL}
PREVIEW_TURN_URL=${PREVIEW_TURN_URL}
LIVEKIT_API_KEY=${LIVEKIT_API_KEY}
LIVEKIT_API_SECRET=${LIVEKIT_API_SECRET}
TURN_SECRET=${TURN_SECRET}
ANGUI_ASR_KEY=${ANGUI_ASR_KEY}
EOF
  flock -u 9

  # A preview is a fresh, disposable environment. Removing its named SQLite
  # volume before starting ensures prior data, sessions, and demo credentials
  # cannot survive into the next deployment of the same preview.
  compose down --volumes --remove-orphans
  compose up --detach --force-recreate --remove-orphans
  for _ in $(seq 1 30); do
    api_id="$(compose ps --quiet api)"
    if [[ -n "${api_id}" ]] && [[ "$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' "${api_id}")" == "healthy" ]]; then
      return 0
    fi
    sleep 2
  done
  compose logs
  die "API did not become healthy"
}

cleanup() {
  require PREVIEW_ROOT
  require PREVIEW_ID
  require PREVIEW_PROJECT

  PREVIEW_DIR="${PREVIEW_ROOT}/${PREVIEW_ID}"
  ensure_preview_dir
  install -d -m 750 "${PREVIEW_ROOT}"
  exec 9>"${PREVIEW_ROOT}/.voice-ports.lock"
  flock -x 9
  if [[ -f "${PREVIEW_DIR}/compose.yml" ]]; then
    compose down --volumes --remove-orphans
  fi
  rm -rf -- "${PREVIEW_DIR}"
  flock -u 9
}

case "${1:-}" in
  deploy) deploy ;;
  cleanup) cleanup ;;
  *) die "usage: $0 deploy|cleanup" ;;
esac
