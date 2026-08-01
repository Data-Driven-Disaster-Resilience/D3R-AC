//! This crate has no library code of its own -- it exists purely to
//! hold `tests/integration_tests.rs`, kept as its own workspace
//! package (rather than a `tests/` directory inside `risk-registry`
//! itself) specifically so that running its tests never requires
//! compiling `risk-registry`'s own `#![no_std]` `[[bin]]` for the
//! native host target.
//!
//! That distinction matters: `risk-registry/src/main.rs` defines its
//! own `#[panic_handler]` (needed for its `wasm32-unknown-unknown`
//! build, since its default allocator/panic-handler feature is
//! disabled -- see that package's Cargo.toml for why). Building that
//! same source for a native target (which `cargo test` does by default
//! for every workspace member's every target, unless scoped away)
//! collides with the native toolchain's own built-in panic handler --
//! confirmed via a real CI error: "error[E0152]: found duplicate lang
//! item `panic_impl`". This crate's tests instead load the already-
//! compiled `risk-registry.wasm` from disk and exercise it through
//! `casper-engine-test-support`'s execution engine, the same pattern
//! every real-world Casper contract test suite in the ecosystem uses
//! (see e.g. casper-ecosystem/erc20's separate `erc20-test`/
//! `erc20-test-call` packages) -- never recompiling the contract's own
//! source for anything other than the wasm32 target.
