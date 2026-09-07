#![no_std]

mod errors;
mod types;

#[cfg(test)]
mod test;

use errors::Error;
use soroban_sdk::{contract, contractimpl, panic_with_error, token, Address, Env, String, Vec};
use types::{DataKey, Escrow, EscrowStatus, Milestone, MilestoneStatus};

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

        milestone.status = MilestoneStatus::Released;
        escrow.milestones.set(milestone_id, milestone);

        let all_released = escrow
            .milestones
            .iter()
            .all(|m| m.status == MilestoneStatus::Released);
        if all_released {
            escrow.status = EscrowStatus::Completed;
        }
        Self::save_escrow(&env, &escrow);
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
    }

    /// Returns the current state of an escrow.
    pub fn get_escrow(env: Env, escrow_id: u64) -> Escrow {
        Self::load_escrow(&env, escrow_id)
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
}
