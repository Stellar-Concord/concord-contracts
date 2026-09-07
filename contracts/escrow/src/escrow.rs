//! Escrow lifecycle: creation, funding, and cancellation.

use crate::errors::Error;
use crate::events::{EscrowCancelled, EscrowCreated, EscrowFunded};
use crate::state;
use crate::types::{Escrow, EscrowStatus, Milestone, MilestoneStatus};
use crate::{EscrowContract, EscrowContractArgs, EscrowContractClient};
use soroban_sdk::{contractimpl, panic_with_error, token, Address, Env, String, Vec};

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

        let escrow_id = state::next_escrow_id(&env);
        let escrow = Escrow {
            id: escrow_id,
            client,
            provider,
            arbitrator,
            token,
            milestones,
            status: EscrowStatus::Created,
        };
        state::save_escrow(&env, &escrow);

        EscrowCreated {
            escrow_id,
            client: escrow.client.clone(),
            provider: escrow.provider.clone(),
            arbitrator: escrow.arbitrator.clone(),
            token: escrow.token.clone(),
            milestones: escrow.milestones.clone(),
        }
        .publish(&env);

        escrow_id
    }

    /// Client deposits the full escrow amount (sum of all milestone amounts)
    /// into the contract. Authorized by the client.
    pub fn fund_escrow(env: Env, escrow_id: u64) {
        let mut escrow = state::load_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Created {
            panic_with_error!(&env, Error::InvalidEscrowStatus);
        }

        escrow.client.require_auth();

        let total = state::total_amount(&env, &escrow.milestones);

        // Update state before the external token call: the token address is
        // caller-supplied and could belong to a contract that calls back
        // into us during `transfer`, so we don't want to still look
        // `Created` (fundable again) if that happens.
        escrow.status = EscrowStatus::Funded;
        state::save_escrow(&env, &escrow);

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(&escrow.client, env.current_contract_address(), &total);

        EscrowFunded { escrow_id, total }.publish(&env);
    }

    /// Cancels an escrow before it has been funded. Authorized by the client.
    pub fn cancel_escrow(env: Env, escrow_id: u64) {
        let mut escrow = state::load_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Created {
            panic_with_error!(&env, Error::InvalidEscrowStatus);
        }

        escrow.client.require_auth();

        escrow.status = EscrowStatus::Cancelled;
        state::save_escrow(&env, &escrow);

        EscrowCancelled { escrow_id }.publish(&env);
    }

    /// Returns the current state of an escrow.
    pub fn get_escrow(env: Env, escrow_id: u64) -> Escrow {
        state::load_escrow(&env, escrow_id)
    }
}
