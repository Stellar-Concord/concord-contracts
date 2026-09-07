//! Storage access. Every read/write to contract state goes through here so
//! the domain modules (`escrow`, `milestone`, `dispute`) stay focused on
//! business rules rather than storage-key bookkeeping.

use crate::errors::Error;
use crate::types::{DataKey, Dispute, Escrow, Milestone, MilestoneStatus};
use soroban_sdk::{panic_with_error, Env, Vec};

pub(crate) fn next_escrow_id(env: &Env) -> u64 {
    let id = env
        .storage()
        .instance()
        .get(&DataKey::EscrowCounter)
        .unwrap_or(0u64);
    env.storage()
        .instance()
        .set(&DataKey::EscrowCounter, &(id + 1));
    id
}

pub(crate) fn load_escrow(env: &Env, escrow_id: u64) -> Escrow {
    env.storage()
        .persistent()
        .get(&DataKey::Escrow(escrow_id))
        .unwrap_or_else(|| panic_with_error!(env, Error::EscrowNotFound))
}

pub(crate) fn save_escrow(env: &Env, escrow: &Escrow) {
    env.storage()
        .persistent()
        .set(&DataKey::Escrow(escrow.id), escrow);
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
    env.storage()
        .persistent()
        .set(&DataKey::Dispute(escrow_id, dispute.milestone_id), dispute);
}
