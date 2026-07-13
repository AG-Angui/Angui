# GitHub workflows

## CI

`ci.yml` runs on every branch push and pull request. Rust formatting, Clippy,
tests, and the release build run in parallel with the frontend lint/build and
live PostgreSQL/MySQL migration checks. The database job applies every
migration, reports status, performs a full refresh, and checks status again for
both server databases. Cargo and npm caches are enabled, and successful
binaries/bundles are retained for seven days.

## Gemini reviewer

Create a Google AI Studio API key and add it as the repository secret
`GEMINI_API_KEY`. The default model is `gemini-2.5-flash`; override it with the
repository variable `GEMINI_REVIEW_MODEL`. `GEMINI_API_BASE` is optional.

The workflow uses `pull_request_target` so fork pull requests can be reviewed,
but it checks out only the trusted default branch. Pull request code is fetched
as plain diff text and is never executed. The reviewer updates one persistent
PR comment instead of creating a new comment for every commit.

Gemini's free API tier has model-specific request/token quotas and can change.
For a small repository, one review per PR update normally fits; very large or
frequently updated diffs may hit rate limits. `REVIEW_MAX_DIFF_CHARS` defaults
to `120000` and can be lowered to control token use.

## Local API reviewer

Install a Linux x64 self-hosted GitHub runner on a machine that can reach the
local API. Configure:

- Secret `LOCAL_REVIEW_API_BASE`: OpenAI-compatible base URL, for example
  `http://127.0.0.1:8000/v1`.
- Secret `LOCAL_REVIEW_API_KEY`: optional bearer token.
- Variable `LOCAL_REVIEW_MODEL`: optional model name; defaults to
  `qwen3-coder`.

Create and add the `local-review` label to a PR to enable this reviewer. It can
also be run manually with a PR number.

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

The backend artifact and image contain both `angui` and `migration`. Compose
runs the one-shot migration service against the branch preview's persistent
SQLite volume before starting the API, keeping application startup separate
from schema management.

Required deployment secrets:

- `PREVIEW_SSH_HOST`
- `PREVIEW_SSH_USER`
- `PREVIEW_SSH_KEY`
- `PREVIEW_GHCR_USERNAME`
- `PREVIEW_GHCR_TOKEN` with `read:packages`

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
