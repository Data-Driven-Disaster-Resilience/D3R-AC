//! See risk-registry-tests/src/lib.rs's module comment for why this
//! crate exists as its own empty-library workspace package (separate
//! from `identity-registry` itself) -- identical reasoning applies
//! here: avoids ever compiling `identity-registry`'s own `#![no_std]`
//! `[[bin]]` for a native target, which would collide with the native
//! toolchain's own panic handler.
