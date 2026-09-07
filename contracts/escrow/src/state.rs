//! Storage access. Every read/write to contract state goes through here so
//! the domain modules (`escrow`, `milestone`, `dispute`) stay focused on
//! business rules rather than storage-key bookkeeping.

use crate::errors::Error;
use crate::types::{DataKey, Dispute, Escrow, Milestone, MilestoneStatus, Resolution};
use soroban_sdk::{panic_with_error, Env, Vec};

/// Stellar ledgers close roughly every 5 seconds.
const DAY_IN_LEDGERS: u32 = 17_280;
/// Bump a persistent entry's TTL once it has less than this many ledgers left.
const TTL_THRESHOLD: u32 = DAY_IN_LEDGERS * 30;
/// ...out to this many ledgers from the current one. An escrow that sits
/// untouched for longer than this between actions would need an external
/// keep-alive extension (e.g. via the `stellar contract extend` CLI) or its
/// data risks archival, at which point it needs restoring before further
/// use. 120 days comfortably covers realistic milestone timelines while
/// staying well under any network's configured max entry TTL.
const TTL_EXTEND_TO: u32 = DAY_IN_LEDGERS * 120;

/// Bumps the contract instance's own TTL. Called on every write so the
/// instance itself (and therefore the whole contract) doesn't expire during
/// a quiet period with no new escrows being created.
fn bump_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
}

pub(crate) fn next_escrow_id(env: &Env) -> u64 {
    let id = env
        .storage()
        .instance()
        .get(&DataKey::EscrowCounter)
        .unwrap_or(0u64);
    env.storage()
        .instance()
        .set(&DataKey::EscrowCounter, &(id + 1));
    bump_instance_ttl(env);
    id
}

pub(crate) fn load_escrow(env: &Env, escrow_id: u64) -> Escrow {
    env.storage()
        .persistent()
        .get(&DataKey::Escrow(escrow_id))
        .unwrap_or_else(|| panic_with_error!(env, Error::EscrowNotFound))
}

pub(crate) fn save_escrow(env: &Env, escrow: &Escrow) {
    let key = DataKey::Escrow(escrow.id);
    env.storage().persistent().set(&key, escrow);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
    bump_instance_ttl(env);
}

pub(crate) fn get_milestone(env: &Env, escrow: &Escrow, milestone_id: u32) -> Milestone {
    escrow
        .milestones
        .get(milestone_id)
        .unwrap_or_else(|| panic_with_error!(env, Error::MilestoneNotFound))
}

pub(crate) fn total_amount(env: &Env, milestones: &Vec<Milestone>) -> i128 {
    let mut total: i128 = 0;
    for m in milestones.iter() {
        total = match total.checked_add(m.amount) {
            Some(t) => t,
            None => panic_with_error!(env, Error::AmountOverflow),
        };
    }
    total
}

pub(crate) fn all_milestones_settled(milestones: &Vec<Milestone>) -> bool {
    milestones
        .iter()
        .all(|m| m.status == MilestoneStatus::Released || m.status == MilestoneStatus::Resolved)
}

pub(crate) fn apply_bps(env: &Env, amount: i128, bps: u32) -> i128 {
    let scaled = match amount.checked_mul(bps as i128) {
        Some(v) => v,
        None => panic_with_error!(env, Error::AmountOverflow),
    };
    scaled / 10_000
}

pub(crate) fn load_dispute(env: &Env, escrow_id: u64, milestone_id: u32) -> Dispute {
    env.storage()
        .persistent()
        .get(&DataKey::Dispute(escrow_id, milestone_id))
        .unwrap_or_else(|| panic_with_error!(env, Error::DisputeNotFound))
}

pub(crate) fn save_dispute(env: &Env, dispute: &Dispute, escrow_id: u64) {
    let key = DataKey::Dispute(escrow_id, dispute.milestone_id);
    env.storage().persistent().set(&key, dispute);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
    bump_instance_ttl(env);
}

pub(crate) fn load_dispute_resolution(env: &Env, escrow_id: u64, milestone_id: u32) -> Resolution {
    env.storage()
        .persistent()
        .get(&DataKey::DisputeResolution(escrow_id, milestone_id))
        .unwrap_or_else(|| panic_with_error!(env, Error::DisputeNotFound))
}

pub(crate) fn save_dispute_resolution(
    env: &Env,
    escrow_id: u64,
    milestone_id: u32,
    resolution: &Resolution,
) {
    let key = DataKey::DisputeResolution(escrow_id, milestone_id);
    env.storage().persistent().set(&key, resolution);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
    bump_instance_ttl(env);
}
