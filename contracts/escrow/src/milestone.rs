//! Milestone submission and approval.

use crate::errors::Error;
use crate::events::{EscrowCompleted, MilestoneApproved, MilestoneSubmitted};
use crate::state;
use crate::types::{EscrowStatus, MilestoneStatus};
use crate::{EscrowContract, EscrowContractArgs, EscrowContractClient};
use soroban_sdk::{contractimpl, panic_with_error, token, Env};

#[contractimpl]
impl EscrowContract {
    /// Provider marks a milestone as delivered. Authorized by the provider.
    pub fn submit_milestone(env: Env, escrow_id: u64, milestone_id: u32) {
        let mut escrow = state::load_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Funded && escrow.status != EscrowStatus::InProgress {
            panic_with_error!(&env, Error::InvalidEscrowStatus);
        }

        escrow.provider.require_auth();

        let mut milestone = state::get_milestone(&env, &escrow, milestone_id);
        if milestone.status != MilestoneStatus::Pending {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }
        milestone.status = MilestoneStatus::Submitted;
        escrow.milestones.set(milestone_id, milestone);

        if escrow.status == EscrowStatus::Funded {
            escrow.status = EscrowStatus::InProgress;
        }
        state::save_escrow(&env, &escrow);

        MilestoneSubmitted {
            escrow_id,
            milestone_id,
        }
        .publish(&env);
    }

    /// Client approves a submitted milestone, releasing its funds to the
    /// provider. Authorized by the client.
    pub fn approve_milestone(env: Env, escrow_id: u64, milestone_id: u32) {
        let mut escrow = state::load_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::InProgress {
            panic_with_error!(&env, Error::InvalidEscrowStatus);
        }

        escrow.client.require_auth();

        let mut milestone = state::get_milestone(&env, &escrow, milestone_id);
        if milestone.status != MilestoneStatus::Submitted {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }

        // Update state before the external token call: the token address is
        // caller-supplied and could belong to a contract that calls back
        // into us during `transfer`. Reading this milestone as `Released`
        // already, rather than still `Submitted`, is what stops a reentrant
        // call from being paid twice for it.
        let amount = milestone.amount;
        milestone.status = MilestoneStatus::Released;
        escrow.milestones.set(milestone_id, milestone);

        let completed = state::all_milestones_settled(&escrow.milestones);
        if completed {
            escrow.status = EscrowStatus::Completed;
        }
        state::save_escrow(&env, &escrow);

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(&env.current_contract_address(), &escrow.provider, &amount);

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
}
