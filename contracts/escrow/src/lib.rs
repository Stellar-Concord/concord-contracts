#![no_std]

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
