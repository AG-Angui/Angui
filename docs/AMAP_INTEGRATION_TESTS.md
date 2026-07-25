# AMap Integration Test

The ordinary Rust test suite never requires a mapping credential and does not
contact AMap. It uses the disabled service and validates the application-level
fallback behavior instead.

`amap_service::tests::live_route_estimates_work_with_an_explicit_non_production_key`
is an opt-in upstream integration test. It uses two public, nearby Beijing
landmarks expressed in GCJ-02 and verifies that AMap accepts the credential and
returns positive distance and duration values for walking and driving routes.
With `--nocapture`, successful runs print only the provider, route mode,
distance, and duration as proof of the upstream response mapping. They never
print credentials, URLs, coordinates, or raw response bodies.

Run it locally only with a restricted non-production AMap Web Service key. The
test and server load the local ignored `.env` file automatically; an exported
process environment variable still takes precedence:

```powershell
# .env (never commit it)
# AMAP_WEBSERVICE_KEY=your-restricted-non-production-key
cargo test live_route_estimates_work_with_an_explicit_non_production_key --locked -- --ignored --nocapture
```

The test is marked `ignored`, so it cannot accidentally run during the ordinary
test suite. Once explicitly invoked, absence of `AMAP_WEBSERVICE_KEY` is a test
failure rather than a silent skip. This keeps normal CI offline,
credential-free, and independent of AMap quotas. Do not add a key to
`.env.example`, GitHub Actions, logs,
screenshots, URLs, or test fixtures.

The test uses the production HTTPS base URL intentionally. Override the client
base URL only in local mocked tests. When a real test fails, verify the key's
Web Service route entitlement, quota, and IP restrictions first; the test never
prints the key or the upstream payload.
