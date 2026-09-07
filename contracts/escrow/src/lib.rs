#![no_std]

// `proptest` (used only under `test/property.rs`) needs `std`. This has no
// effect on the wasm build: it's gated to `cfg(test)`, which never applies
// to a `stellar contract build`.
#[cfg(test)]
extern crate std;

mod dispute;
mod errors;
mod escrow;
mod events;
mod milestone;
mod state;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::contract;

/// Milestone-based escrow. Contract functions are grouped by domain across
/// `escrow.rs` (lifecycle: create/fund/cancel), `milestone.rs` (submit/
/// approve), and `dispute.rs` (raise/resolve) — each its own
/// `#[contractimpl]` block for this type, which Soroban allows and merges
/// into a single exported contract.
#[contract]
pub struct EscrowContract;
