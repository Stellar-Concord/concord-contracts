//! Dispute lifecycle: raising and arbitrator resolution.

use crate::errors::Error;
use crate::events::{DisputeRaised, DisputeResolved, EscrowCompleted};
use crate::state;
use crate::types::{DataKey, Dispute, EscrowStatus, MilestoneStatus, Resolution};
use crate::{EscrowContract, EscrowContractArgs, EscrowContractClient};
use soroban_sdk::{contractimpl, panic_with_error, token, Address, Env, String};

#[contractimpl]
impl EscrowContract {
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
        let mut escrow = state::load_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Funded && escrow.status != EscrowStatus::InProgress {
            panic_with_error!(&env, Error::InvalidEscrowStatus);
        }

        raised_by.require_auth();
        if raised_by != escrow.client && raised_by != escrow.provider {
            panic_with_error!(&env, Error::NotDisputeParty);
        }

        let mut milestone = state::get_milestone(&env, &escrow, milestone_id);
        if milestone.status != MilestoneStatus::Pending
            && milestone.status != MilestoneStatus::Submitted
        {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }
        milestone.status = MilestoneStatus::Disputed;
        escrow.milestones.set(milestone_id, milestone);
        state::save_escrow(&env, &escrow);

        state::save_dispute(
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
        let mut escrow = state::load_escrow(&env, escrow_id);
        escrow.arbitrator.require_auth();

        let mut milestone = state::get_milestone(&env, &escrow, milestone_id);
        if milestone.status != MilestoneStatus::Disputed {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }
        state::load_dispute(&env, escrow_id, milestone_id);

        // Validate the resolution and work out who gets paid what before
        // touching storage or calling out to the token contract.
        let amount = milestone.amount;
        let (provider_amount, client_amount): (i128, i128) = match &resolution {
            Resolution::ReleaseToProvider => (amount, 0),
            Resolution::RefundToClient => (0, amount),
            Resolution::Split(provider_bps) => {
                if *provider_bps > 10_000 {
                    panic_with_error!(&env, Error::InvalidSplitPercentage);
                }
                let provider_amount = state::apply_bps(&env, amount, *provider_bps);
                (provider_amount, amount - provider_amount)
            }
        };

        // Update state before the external token calls: the token address
        // is caller-supplied and could belong to a contract that calls back
        // into us during `transfer`. Reading this milestone as `Resolved`
        // already, rather than still `Disputed`, is what stops a reentrant
        // call from being settled twice.
        milestone.status = MilestoneStatus::Resolved;
        escrow.milestones.set(milestone_id, milestone);

        env.storage().persistent().set(
            &DataKey::DisputeResolution(escrow_id, milestone_id),
            &resolution,
        );

        let completed = state::all_milestones_settled(&escrow.milestones);
        if completed {
            escrow.status = EscrowStatus::Completed;
        }
        state::save_escrow(&env, &escrow);

        let token_client = token::Client::new(&env, &escrow.token);
        let contract_address = env.current_contract_address();
        if provider_amount > 0 {
            token_client.transfer(&contract_address, &escrow.provider, &provider_amount);
        }
        if client_amount > 0 {
            token_client.transfer(&contract_address, &escrow.client, &client_amount);
        }

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

    /// Returns the dispute record for a milestone.
    pub fn get_dispute(env: Env, escrow_id: u64, milestone_id: u32) -> Dispute {
        state::load_dispute(&env, escrow_id, milestone_id)
    }

    /// Returns the resolution chosen for a resolved dispute.
    pub fn get_dispute_resolution(env: Env, escrow_id: u64, milestone_id: u32) -> Resolution {
        env.storage()
            .persistent()
            .get(&DataKey::DisputeResolution(escrow_id, milestone_id))
            .unwrap_or_else(|| panic_with_error!(&env, Error::DisputeNotFound))
    }
}
