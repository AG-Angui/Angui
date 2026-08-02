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

  PREVIEW_DIR="${PREVIEW_ROOT}/${PREVIEW_ID}"
  ensure_preview_dir
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
EOF

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
  if [[ -f "${PREVIEW_DIR}/compose.yml" ]]; then
    compose down --volumes --remove-orphans
  fi
  rm -rf -- "${PREVIEW_DIR}"
}

case "${1:-}" in
  deploy) deploy ;;
  cleanup) cleanup ;;
  *) die "usage: $0 deploy|cleanup" ;;
esac
