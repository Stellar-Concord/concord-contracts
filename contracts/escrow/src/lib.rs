#![no_std]

mod errors;
mod events;
mod types;

#[cfg(test)]
mod test;

use errors::Error;
use events::{
    DisputeRaised, DisputeResolved, EscrowCancelled, EscrowCompleted, EscrowCreated, EscrowFunded,
    MilestoneApproved, MilestoneSubmitted,
};
use soroban_sdk::{contract, contractimpl, panic_with_error, token, Address, Env, String, Vec};
use types::{DataKey, Dispute, Escrow, EscrowStatus, Milestone, MilestoneStatus, Resolution};

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    /// Creates a new escrow with the given milestones. Authorized by the client.
    /// Returns the new escrow's id.
    pub fn initialize_escrow(
        env: Env,
        client: Address,
        provider: Address,
        arbitrator: Address,
        token: Address,
        milestone_descriptions: Vec<String>,
        milestone_amounts: Vec<i128>,
    ) -> u64 {
        client.require_auth();

        if milestone_descriptions.is_empty() {
            panic_with_error!(&env, Error::NoMilestones);
        }
        if milestone_descriptions.len() != milestone_amounts.len() {
            panic_with_error!(&env, Error::MismatchedMilestoneInputs);
        }

        let mut milestones = Vec::new(&env);
        for i in 0..milestone_descriptions.len() {
            let amount = milestone_amounts.get(i).unwrap();
            if amount <= 0 {
                panic_with_error!(&env, Error::InvalidMilestoneAmount);
            }
            milestones.push_back(Milestone {
                id: i,
                description: milestone_descriptions.get(i).unwrap(),
                amount,
                status: MilestoneStatus::Pending,
            });
        }

        let escrow_id = Self::next_escrow_id(&env);
        let escrow = Escrow {
            id: escrow_id,
            client,
            provider,
            arbitrator,
            token,
            milestones,
            status: EscrowStatus::Created,
        };
        Self::save_escrow(&env, &escrow);

        EscrowCreated {
            escrow_id,
            client: escrow.client.clone(),
            provider: escrow.provider.clone(),
            arbitrator: escrow.arbitrator.clone(),
            token: escrow.token.clone(),
        }
        .publish(&env);

        escrow_id
    }

    /// Client deposits the full escrow amount (sum of all milestone amounts)
    /// into the contract. Authorized by the client.
    pub fn fund_escrow(env: Env, escrow_id: u64) {
        let mut escrow = Self::load_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Created {
            panic_with_error!(&env, Error::InvalidEscrowStatus);
        }

        escrow.client.require_auth();

        let total = Self::total_amount(&env, &escrow.milestones);
        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(&escrow.client, env.current_contract_address(), &total);

        escrow.status = EscrowStatus::Funded;
        Self::save_escrow(&env, &escrow);

        EscrowFunded { escrow_id, total }.publish(&env);
    }

    /// Provider marks a milestone as delivered. Authorized by the provider.
    pub fn submit_milestone(env: Env, escrow_id: u64, milestone_id: u32) {
        let mut escrow = Self::load_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Funded && escrow.status != EscrowStatus::InProgress {
            panic_with_error!(&env, Error::InvalidEscrowStatus);
        }

        escrow.provider.require_auth();

        let mut milestone = Self::get_milestone(&env, &escrow, milestone_id);
        if milestone.status != MilestoneStatus::Pending {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }
        milestone.status = MilestoneStatus::Submitted;
        escrow.milestones.set(milestone_id, milestone);

        if escrow.status == EscrowStatus::Funded {
            escrow.status = EscrowStatus::InProgress;
        }
        Self::save_escrow(&env, &escrow);

        MilestoneSubmitted {
            escrow_id,
            milestone_id,
        }
        .publish(&env);
    }

    /// Client approves a submitted milestone, releasing its funds to the
    /// provider. Authorized by the client.
    pub fn approve_milestone(env: Env, escrow_id: u64, milestone_id: u32) {
        let mut escrow = Self::load_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::InProgress {
            panic_with_error!(&env, Error::InvalidEscrowStatus);
        }

        escrow.client.require_auth();

        let mut milestone = Self::get_milestone(&env, &escrow, milestone_id);
        if milestone.status != MilestoneStatus::Submitted {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.provider,
            &milestone.amount,
        );

        let amount = milestone.amount;
        milestone.status = MilestoneStatus::Released;
        escrow.milestones.set(milestone_id, milestone);

        let completed = Self::all_milestones_settled(&escrow.milestones);
        if completed {
            escrow.status = EscrowStatus::Completed;
        }
        Self::save_escrow(&env, &escrow);

        MilestoneApproved {
            escrow_id,
            milestone_id,
            amount,
        }
        .publish(&env);
        if completed {
            EscrowCompleted { escrow_id }.publish(&env);
        }
    }

    /// Cancels an escrow before it has been funded. Authorized by the client.
    pub fn cancel_escrow(env: Env, escrow_id: u64) {
        let mut escrow = Self::load_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Created {
            panic_with_error!(&env, Error::InvalidEscrowStatus);
        }

        escrow.client.require_auth();

        escrow.status = EscrowStatus::Cancelled;
        Self::save_escrow(&env, &escrow);

        EscrowCancelled { escrow_id }.publish(&env);
    }

    /// Raises a dispute on a milestone that has not yet been released.
    /// Authorized by either the client or the provider. Freezes only the
    /// disputed milestone; other milestones are unaffected.
    pub fn raise_dispute(
        env: Env,
        escrow_id: u64,
        milestone_id: u32,
        raised_by: Address,
        reason: String,
    ) {
        let mut escrow = Self::load_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Funded && escrow.status != EscrowStatus::InProgress {
            panic_with_error!(&env, Error::InvalidEscrowStatus);
        }

        raised_by.require_auth();
        if raised_by != escrow.client && raised_by != escrow.provider {
            panic_with_error!(&env, Error::NotDisputeParty);
        }

        let mut milestone = Self::get_milestone(&env, &escrow, milestone_id);
        if milestone.status != MilestoneStatus::Pending
            && milestone.status != MilestoneStatus::Submitted
        {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }
        milestone.status = MilestoneStatus::Disputed;
        escrow.milestones.set(milestone_id, milestone);
        Self::save_escrow(&env, &escrow);

        Self::save_dispute(
            &env,
            &Dispute {
                milestone_id,
                raised_by: raised_by.clone(),
                reason: reason.clone(),
            },
            escrow_id,
        );

        DisputeRaised {
            escrow_id,
            milestone_id,
            raised_by,
            reason,
        }
        .publish(&env);
    }

    /// Resolves a disputed milestone, settling its funds according to
    /// `resolution`. Authorized by the escrow's arbitrator.
    pub fn resolve_dispute(env: Env, escrow_id: u64, milestone_id: u32, resolution: Resolution) {
        let mut escrow = Self::load_escrow(&env, escrow_id);
        escrow.arbitrator.require_auth();

        let mut milestone = Self::get_milestone(&env, &escrow, milestone_id);
        if milestone.status != MilestoneStatus::Disputed {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }
        Self::load_dispute(&env, escrow_id, milestone_id);

        let token_client = token::Client::new(&env, &escrow.token);
        let contract_address = env.current_contract_address();
        match &resolution {
            Resolution::ReleaseToProvider => {
                token_client.transfer(&contract_address, &escrow.provider, &milestone.amount);
            }
            Resolution::RefundToClient => {
                token_client.transfer(&contract_address, &escrow.client, &milestone.amount);
            }
            Resolution::Split(provider_bps) => {
                if *provider_bps > 10_000 {
                    panic_with_error!(&env, Error::InvalidSplitPercentage);
                }
                let provider_amount = Self::apply_bps(&env, milestone.amount, *provider_bps);
                let client_amount = milestone.amount - provider_amount;
                if provider_amount > 0 {
                    token_client.transfer(&contract_address, &escrow.provider, &provider_amount);
                }
                if client_amount > 0 {
                    token_client.transfer(&contract_address, &escrow.client, &client_amount);
                }
            }
        }

        milestone.status = MilestoneStatus::Resolved;
        escrow.milestones.set(milestone_id, milestone);

        env.storage().persistent().set(
            &DataKey::DisputeResolution(escrow_id, milestone_id),
            &resolution,
        );

        let completed = Self::all_milestones_settled(&escrow.milestones);
        if completed {
            escrow.status = EscrowStatus::Completed;
        }
        Self::save_escrow(&env, &escrow);

        DisputeResolved {
            escrow_id,
            milestone_id,
            resolution,
        }
        .publish(&env);
        if completed {
            EscrowCompleted { escrow_id }.publish(&env);
        }
    }

    /// Returns the current state of an escrow.
    pub fn get_escrow(env: Env, escrow_id: u64) -> Escrow {
        Self::load_escrow(&env, escrow_id)
    }

    /// Returns the dispute record for a milestone.
    pub fn get_dispute(env: Env, escrow_id: u64, milestone_id: u32) -> Dispute {
        Self::load_dispute(&env, escrow_id, milestone_id)
    }

    /// Returns the resolution chosen for a resolved dispute.
    pub fn get_dispute_resolution(env: Env, escrow_id: u64, milestone_id: u32) -> Resolution {
        env.storage()
            .persistent()
            .get(&DataKey::DisputeResolution(escrow_id, milestone_id))
            .unwrap_or_else(|| panic_with_error!(&env, Error::DisputeNotFound))
    }

    fn next_escrow_id(env: &Env) -> u64 {
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

    fn load_escrow(env: &Env, escrow_id: u64) -> Escrow {
        env.storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::EscrowNotFound))
    }

    fn save_escrow(env: &Env, escrow: &Escrow) {
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(escrow.id), escrow);
    }

    fn get_milestone(env: &Env, escrow: &Escrow, milestone_id: u32) -> Milestone {
        escrow
            .milestones
            .get(milestone_id)
            .unwrap_or_else(|| panic_with_error!(env, Error::MilestoneNotFound))
    }

    fn total_amount(env: &Env, milestones: &Vec<Milestone>) -> i128 {
        let mut total: i128 = 0;
        for m in milestones.iter() {
            total = match total.checked_add(m.amount) {
                Some(t) => t,
                None => panic_with_error!(env, Error::AmountOverflow),
            };
        }
        total
    }

    fn all_milestones_settled(milestones: &Vec<Milestone>) -> bool {
        milestones
            .iter()
            .all(|m| m.status == MilestoneStatus::Released || m.status == MilestoneStatus::Resolved)
    }

    fn apply_bps(env: &Env, amount: i128, bps: u32) -> i128 {
        let scaled = match amount.checked_mul(bps as i128) {
            Some(v) => v,
            None => panic_with_error!(env, Error::AmountOverflow),
        };
        scaled / 10_000
    }

    fn load_dispute(env: &Env, escrow_id: u64, milestone_id: u32) -> Dispute {
        env.storage()
            .persistent()
            .get(&DataKey::Dispute(escrow_id, milestone_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::DisputeNotFound))
    }

    fn save_dispute(env: &Env, dispute: &Dispute, escrow_id: u64) {
        env.storage()
            .persistent()
            .set(&DataKey::Dispute(escrow_id, dispute.milestone_id), dispute);
    }
}
