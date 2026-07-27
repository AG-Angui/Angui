# GitHub workflows

## CI

`ci.yml` runs on every branch push and pull request. Its concurrency identity
is the head commit SHA rather than the branch or PR number. When `push` and
`pull_request` refer to the same SHA, the first run performs the full CI while
the second waits, detects the successful run for that SHA, and skips all
expensive jobs. New branches therefore still receive CI before a PR exists,
while a PR update does not compile, test, migrate, or upload artifacts twice.
Rust formatting, Clippy, tests, and the release build run in parallel with the
frontend lint/build and live PostgreSQL/MySQL migration checks. The database job
applies every migration, reports status, performs a full refresh, and checks
status again for both server databases. Cargo and Yarn caches are enabled, and
successful binaries/bundles are retained for seven days.

## Pull request quality

`quality-pr.yml` is the Rust/Actions adaptation of the reference repository's
Go-oriented workflow. It checks `cargo fmt` and validates GitHub Actions syntax,
expressions, and job wiring with actionlint. Clippy, tests, and release builds
remain in `ci.yml` so Rust is not compiled twice merely to produce inline
review comments.

`zizmor.yml` audits workflow security whenever `.github/workflows/**` changes.
It checks dangerous triggers, token permissions, action pinning, template
injection, and other GitHub Actions security smells.

OpenAPI linting is intentionally not enabled yet because the repository has no
OpenAPI document or generator. Add it when a committed `openapi.yaml` or
equivalent generated contract becomes part of the project.

CodeQL is intentionally not enabled while the repository remains UNLICENSED.
GitHub currently supports Rust, JavaScript/TypeScript, and Actions analysis, but
private/non-open-source use requires an appropriate GitHub Code Security or
Advanced Security license. Revisit CodeQL after adopting an OSI-approved
license for a public repository or enabling the paid product.

## Security checks

`security.yml` runs on pull requests, pushes to `main`, manual dispatch, and
weekly at 03:17 UTC on Monday. It fails on RustSec advisories in `Cargo.lock`,
high or critical Yarn advisories in the frontend lockfile, and detected secrets.
Routine runs scan the working tree; the weekly run scans all Git history.

The frontend lockfile downloads packages through npmmirror, which does not
implement its advisory endpoint. The workflow intentionally sends only the Yarn
audit request to `registry.npmjs.org`; ordinary CI dependency installation uses
the configured Chinese mirror. Trivy image scanning belongs after preview images
are enabled, and `cargo-deny` should be added after third-party license/source
policy is defined.

## Gemini Code Assist GitHub app

There is no repository-managed Gemini review workflow. Gemini Code Assist can
be installed as a GitHub integration and configured outside Actions. However,
Google's current documentation says the consumer version will stop serving
requests on July 17, 2026 and should not be newly installed. Use the enterprise
Google Cloud integration or the custom API reviewer below for a durable setup.

Official documentation:
<https://docs.cloud.google.com/gemini/docs/code-review/review-repo-code>

## Custom API reviewer

`custom-api-review.yml` calls any reachable OpenAI-compatible API. It defaults
to a GitHub-hosted runner; the API does not need to be local. Configure:

- Secret `CUSTOM_REVIEW_API_BASE`: API base URL, for example
  `https://ai.example.com/v1` or a full `/chat/completions` URL.
- Secret `CUSTOM_REVIEW_API_KEY`: optional bearer token.
- Variable `CUSTOM_REVIEW_MODEL`: optional model name; defaults to
  `qwen3-coder`.
- Variable `CUSTOM_REVIEW_API_STYLE`: optional. Defaults to `chat` for
  OpenAI-compatible Chat Completions APIs. Set `responses` for OpenAI's
  Responses API.
- Variable `CUSTOM_REVIEW_REASONING_EFFORT`: optional Responses API reasoning
  effort; defaults to `medium` and is ignored in `chat` mode.
- Variable `CUSTOM_REVIEW_MAX_DIFF_CHARS`: optional diff limit; defaults to
  `120000`.
- Variable `CUSTOM_REVIEW_LABEL`: optional. When empty, review every non-draft
  PR; when set, only review PRs carrying that label.
- Variable `CUSTOM_REVIEW_RUNNER`: optional runner label. Defaults to
  `ubuntu-latest`; set it to a self-hosted custom label such as `angui-review`
  when the API is only reachable from a private network.

The workflow never checks out pull request code. It reads the diff through the
GitHub API and updates one persistent PR comment. It can also be run manually
with a PR number. The configured API receives the PR title, branch names, and up
to the configured number of diff characters, so use a provider whose retention
and training policy is acceptable for the repository.

For OpenAI Responses API with GPT-5.6, configure
`CUSTOM_REVIEW_API_BASE=https://api.openai.com/v1`,
`CUSTOM_REVIEW_API_STYLE=responses`, and `CUSTOM_REVIEW_MODEL=gpt-5.6`.
The workflow sends `instructions`, `input`, and `reasoning.effort`, and parses
review text from `output[*].content[*]` entries of type `output_text`.

## Self-hosted runner

Create a dedicated, non-root Linux user and open the repository's **Settings >
Actions > Runners > New self-hosted runner** page. Select Linux/x64 and run the
download and `config.sh` commands shown by GitHub; the registration token is
short-lived, so copy it from that page instead of storing it in the repository.

Register the runner with the CI and preview labels:

```bash
./config.sh --url https://github.com/AG-Angui/Angui \
  --token '<short-lived-registration-token>' \
  --labels angui-ci,angui-preview \
  --unattended
sudo ./svc.sh install
sudo ./svc.sh start
sudo ./svc.sh status
```

The `angui-ci` label runs the CI workflow. The `angui-preview` label deploys
previews locally on this server and builds release images only when a `v*` tag
is pushed. A runner used for CI or previews needs Docker Engine and Docker
Compose v2. The database CI job starts PostgreSQL and MySQL service containers,
so Docker must be available to the runner service account. CI jobs force
external fork pull requests back to `ubuntu-latest`, because they checkout and
execute untrusted PR code.

This intentionally places internal CI and preview deployment on one Docker-capable
host. Anyone who can push a branch in this repository can make CI execute code
there, and Docker access is effectively host-level access. Grant branch push
permission only to trusted maintainers; use a separate, non-deployment runner
if that trust boundary changes.

CI dependency downloads use the repository-managed Chinese mirrors: Rustup and
Cargo use `rsproxy.cn`, while Node.js and Yarn use `npmmirror.com`. Docker image
pulls remain governed by the Docker daemon configuration on the server. The
self-hosted runner must still be able to reach GitHub Actions and GHCR through
direct Internet access or a standard HTTPS proxy; URL-prefix download
accelerators are not sufficient for a runner.

## Preview domains and releases

Branch and internal pull request previews reuse successful CI artifacts. A
branch without an associated open PR receives its branch preview; once a push
is associated with an internal PR, the same successful push CI supplies the PR
preview artifacts and the branch preview is skipped.
They mount the `angui`, `angui-admin`, `migration`, and frontend `dist` files
into one fixed local runtime image; they never build or push a new application
image for each commit. The local runtime image is automatically built once as
`angui-preview-runtime:bookworm` if it does not already exist. Each preview
uses a project-local Docker volume containing a SQLite database shared by the
migration, bootstrap, and API containers; no external database service is
needed for a complete preview.

The VM-local Traefik publishes previews through Docker labels. The parent host's
Nginx owns the wildcard certificate and forwards requests to the VM while
preserving the original `Host` header:

| Kind | URL | Retention |
| --- | --- | --- |
| Branch without an open PR | `<branch-slug>.angui.cg8.site` | Latest commit while the branch exists |
| Internal PR | `pr-<number>-<short-sha>.angui.cg8.site` | Latest commit while the PR is open |
| Tag | `tag-<tag-slug>.angui.cg8.site` | Retained until manually removed |

PR previews are only deployed when the PR head repository is this repository.
Artifacts from external forks never run on the Docker-capable deployment host.
When a PR is closed, `pr-preview-cleanup.yml` runs the trusted default-branch
cleanup definition without checking out PR code.

Every deployment first removes the preview's SQLite volume, then runs
`migration up` and `angui-admin bootstrap-demo`. The workflow creates a new
random password and uses it for the five `.invalid` demo accounts. After the
API health check passes, `pr-preview.yml` posts the preview URL and those
credentials to the PR. A later deployment of that preview invalidates all
previous login sessions, accounts, data, and passwords; old PR comments are
therefore informational only and must not be used as credentials. The Preview
Compose configuration explicitly marks the bootstrap container as `preview`
and enables its one-shot switch; `angui-admin` rejects the command in
production, an unknown environment, or without that switch.

Pushing a tag matching `v*` runs `release.yml`. It runs the full release checks,
builds and pushes immutable `ghcr.io/<repository>-api:<tag>` and
`ghcr.io/<repository>-web:<tag>` images plus `latest`, then deploys the
permanent tag URL from the same release artifacts. Tag preview directories are
not deleted by any workflow.

### Server reverse proxy

The parent host's Nginx owns DNS, TLS certificates, and public ports 80/443.
It must forward every preview hostname to this VM's port 80 while preserving
`Host`, `X-Forwarded-For`, and `X-Forwarded-Proto`. On the VM, start the
code-free Traefik router once:

```bash
cd deploy/proxy
docker compose up -d
```

Traefik on the VM creates the `angui-proxy` Docker network and listens only on
port 80; it does not use DNS credentials, ACME, or port 443. Do not publish
individual preview ports.

### GitHub configuration

Required repository variables:

- `PREVIEW_DEPLOY_ENABLED=true`
- `PREVIEW_BASE_DOMAIN=angui.cg8.site`

Recommended variables:

- `PREVIEW_SCHEME=https`
- `PREVIEW_ROOT=/var/github-runner/angui/previews`
- `PREVIEW_PROXY_NETWORK=angui-proxy`
- `PREVIEW_RUNTIME_IMAGE=angui-preview-runtime:bookworm`
- `PREVIEW_ENVIRONMENT=preview`

Optional preview secret:

- `AMAP_WEBSERVICE_KEY`: a restricted, non-production AMap Web Service key.
  Preview deployment passes it only to the API container at runtime. It is not
  exposed to frontend builds, preview artifacts, migrations, bootstrap jobs, or
  workflow logs. When omitted, the API deliberately uses its documented map
  degradation path. Prefer the `preview` GitHub Environment Secret of this
  name, with deployment branch restrictions or required reviewers. A repository
  secret of the same name also works, but grants the credential to every
  eligible preview deployment and is therefore less restrictive.

The VM needs Docker Engine, Compose v2, `curl`, port 80 reachable from the
parent Nginx, and permission for the runner service account to run Docker. No
SSH deployment secrets, GHCR read token, preview port range, DNS credentials, or
`PREVIEW_BUILD_ENABLED` variable are used by the new preview path. Configure the
`preview` GitHub Environment with deployment branch restrictions or reviewers if
needed.

## GitHub Actions quota

For public repositories, standard GitHub-hosted runners are generally free.
For private repositories on GitHub Free, the included Linux runner allowance
has historically been 2,000 minutes per month; check the account billing page
for the current plan value. This project should usually take roughly 5-15
minutes on an empty Rust cache and 2-6 minutes when cached, plus image builds.

CI and previews run on the self-hosted server, so they do not consume
GitHub-hosted runner minutes. CI artifacts still expire after seven days. Only
versioned release images are pushed to GHCR; configure a package retention policy
for `latest` and old release tags according to the project's release policy.
