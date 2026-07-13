# GitHub workflows

## CI

`ci.yml` runs on every branch push and pull request. Rust formatting, Clippy,
tests, and the release build run in parallel with the frontend lint/build and
live PostgreSQL/MySQL migration checks. The database job applies every
migration, reports status, performs a full refresh, and checks status again for
both server databases. Cargo and npm caches are enabled, and successful
binaries/bundles are retained for seven days.

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

## Self-hosted runner

Create a dedicated, non-root Linux user and open the repository's **Settings >
Actions > Runners > New self-hosted runner** page. Select Linux/x64 and run the
download and `config.sh` commands shown by GitHub; the registration token is
short-lived, so copy it from that page instead of storing it in the repository.

Add purpose-specific labels during registration, for example:

```bash
./config.sh --url https://github.com/AG-Angui/Angui \
  --token '<short-lived-registration-token>' \
  --labels angui-ci,angui-review \
  --unattended
sudo ./svc.sh install
sudo ./svc.sh start
sudo ./svc.sh status
```

For the custom reviewer, set `CUSTOM_REVIEW_RUNNER=angui-review`. A runner used
for CI or previews also needs Docker Engine and Docker Compose v2. Keep runner
workloads isolated from production services, do not mount the Docker socket
into untrusted containers, and do not run fork-controlled code on a runner that
holds deployment or API secrets.

The workflows can be switched without editing YAML by setting repository
variables:

- `RUST_CI_RUNNER=angui-ci`
- `DATABASE_CI_RUNNER=angui-ci`
- `FRONTEND_CI_RUNNER=angui-ci`
- `PREVIEW_BUILD_RUNNER=angui-ci`
- `CUSTOM_REVIEW_RUNNER=angui-review`

Leave any variable unset to keep that job on `ubuntu-latest`. The database job
uses PostgreSQL and MySQL service containers, so a self-hosted database runner
must be Linux and have a working Docker daemon. CI jobs force fork pull requests
back to `ubuntu-latest` even when these variables are set, because they checkout
and execute PR code. Repository-internal branches and push runs can use the
self-hosted labels.

## Branch previews

After a successful push-triggered `ci` run, `preview.yml` can build backend and
frontend images and publish SHA and branch tags to GHCR. Image builds and
deployment are off by default so an unconfigured repository does not burn
runner minutes. Set `PREVIEW_BUILD_ENABLED=true` to publish images without a
server, or set `PREVIEW_DEPLOY_ENABLED=true` after preparing a Docker Compose
server (deployment also enables image builds).

The preview workflow downloads the release binary and frontend bundle from the
successful CI run and only packages them into runtime images. It does not
compile Rust a second time.

The backend artifact and image contain `angui`, `angui-admin`, and `migration`.
Compose first runs the one-shot migration service, then explicitly bootstraps
the `.invalid` demo accounts, and only then starts the API. Application startup
therefore remains separate from schema and account initialization. Re-running
the bootstrap revokes prior demo-account sessions.

Required deployment secrets:

- `PREVIEW_SSH_HOST`
- `PREVIEW_SSH_USER`
- `PREVIEW_SSH_KEY`
- `PREVIEW_GHCR_USERNAME`
- `PREVIEW_GHCR_TOKEN` with `read:packages`
- `PREVIEW_DEMO_PASSWORD` with 12-256 characters; used only by the explicit
  one-shot demo account bootstrap service

Optional configuration:

- Secret `PREVIEW_SSH_PORT`, default `22`
- Variable `PREVIEW_ROOT`, default `/opt/angui-previews`
- Variable `PREVIEW_BASE_PORT`, default `20000`; each branch receives a stable
  port in the following 10000-port range
- Variable `PREVIEW_PUBLIC_HOST`, default the SSH host
- Variable `PREVIEW_SCHEME`, default `http`

The server needs Docker Engine, the Compose v2 plugin, `curl`, and permission
for the SSH user to run Docker. Configure the `preview` GitHub Environment with
deployment branch restrictions or reviewers if untrusted collaborators can
push branches. Deleting a branch stops its preview and removes its volume.

## GitHub Actions quota

For public repositories, standard GitHub-hosted runners are generally free.
For private repositories on GitHub Free, the included Linux runner allowance
has historically been 2,000 minutes per month; check the account billing page
for the current plan value. This project should usually take roughly 5-15
minutes on an empty Rust cache and 2-6 minutes when cached, plus image builds.

If every commit runs both CI and preview image builds in a busy private repo,
the included allowance can be exhausted. The current split makes migration to
self-hosted runners straightforward: replace `ubuntu-latest` with a dedicated
label such as `[self-hosted, linux, x64, angui-ci]`. Keep self-hosted runners
off fork-controlled jobs that expose secrets.

Runner minutes are separate from artifact and GHCR storage. CI artifacts expire
after seven days, but immutable `sha-*` container tags accumulate. Configure a
GHCR package retention policy or a periodic package cleanup once preview builds
are enabled.
