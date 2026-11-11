# Temporary SQLx SQLite compatibility patch

This directory contains the source and manifests from the crates.io
`sqlx-sqlite` 0.9.0 package (upstream commit
`003b698e99e024f3621b8043a2426fde5b741171`). `Cargo.toml.orig` is the unmodified
upstream manifest; the normalized `Cargo.toml` is used by Cargo.

The only functional change is the `libsqlite3-sys` requirement in `Cargo.toml`:
`>=0.30.1, <0.38.0` becomes `>=0.39.1, <0.40.0`. This selects bundled SQLite
3.54.1 and fixes RUSTSEC-2026-0459. No Rust driver source is modified.

SQLx's upstream main branch already allows `<0.40.0`, as observed at commit
`fd56e1dffe74d4b6f9e7ce710bac02dd9026258e`. The local minimum additionally
prevents resolving to an affected version when regenerating Cargo.lock.

Remove this directory, the workspace exclusion and `[patch.crates-io]` entry
when a published SQLx release supports libsqlite3-sys 0.39, then update the lockfile
and rerun the workspace tests and cargo-audit. Preserve the patched SQLite minimum.

Upstream: https://github.com/launchbadge/sqlx

License: MIT OR Apache-2.0; see LICENSE-MIT and LICENSE-APACHE.
