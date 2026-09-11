//! See risk-registry-tests/src/lib.rs's module comment for why this
//! crate exists as its own empty-library workspace package -- needs
//! `funding-request-registry.wasm` staged into its own `wasm/`
//! directory before its tests can run (see
//! contracts/casper/README.md's "Building wasm32-unknown-unknown
//! without rustup" section for the exact steps if `rustup` isn't
//! available).
//!
//! This package was missing entirely until now -- the only one of
//! this suite's seven contracts without a corresponding `-tests`
//! package. Written and verified for real in this pass, against
//! actual compiled `.wasm` binaries on `casper-engine-test-support`'s
//! local execution engine, not just checked for syntax.
