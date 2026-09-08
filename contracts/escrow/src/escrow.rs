//! Escrow lifecycle: creation, funding, and cancellation.

use crate::errors::Error;
use crate::events::{EscrowCancelled, EscrowCreated, EscrowFunded, EscrowMutuallyCancelled};
use crate::state;
use crate::types::{Escrow, EscrowStatus, Milestone, MilestoneInput, MilestoneStatus};
use crate::{EscrowContract, EscrowContractArgs, EscrowContractClient};
use soroban_sdk::{contractimpl, panic_with_error, token, Address, BytesN, Env, String, Vec};

#[contractimpl]
impl EscrowContract {
    /// Creates a new escrow with the given milestones. Authorized by the client.
    /// Returns the new escrow's id.
    ///
    /// `review_period` (seconds) is how long the client has, after the
    /// provider submits a milestone, before `auto_release_milestone`
    /// becomes callable on it. It's set once here and applies to every
    /// milestone in this escrow, rather than either a contract-wide
    /// constant or a per-milestone value: a global constant would force
    /// every client relationship into the same review window regardless of
    /// how much either party trusts the other, while a per-milestone value
    /// adds a field nobody asked for -- a client who wants different review
    /// windows for different kinds of work can already get that by using
    /// separate escrows. Must be strictly positive: a zero-second review
    /// period would let a milestone become auto-releasable in the same
    /// instant it's submitted, defeating the point of giving the client a
    /// review window at all.
    pub fn initialize_escrow(
        env: Env,
        client: Address,
        provider: Address,
        arbitrator: Address,
        token: Address,
        milestones: Vec<MilestoneInput>,
        review_period: u64,
    ) -> u64 {
        client.require_auth();

        if client == provider || client == arbitrator || provider == arbitrator {
            panic_with_error!(&env, Error::RolesMustBeDistinct);
        }

        if milestones.is_empty() {
            panic_with_error!(&env, Error::NoMilestones);
        }
        if review_period == 0 {
            panic_with_error!(&env, Error::InvalidReviewPeriod);
        }

        let now = env.ledger().timestamp();
        let mut built_milestones = Vec::new(&env);
        for (i, input) in milestones.iter().enumerate() {
            if input.amount <= 0 {
                panic_with_error!(&env, Error::InvalidMilestoneAmount);
            }
            if input.deadline <= now {
                panic_with_error!(&env, Error::InvalidDeadline);
            }
            built_milestones.push_back(Milestone {
                id: i as u32,
                description: input.description,
                amount: input.amount,
                status: MilestoneStatus::Pending,
                deadline: input.deadline,
                submitted_at: 0,
                evidence_uri: String::from_str(&env, ""),
                evidence_hash: BytesN::from_array(&env, &[0u8; 32]),
            });
        }

        let escrow_id = state::next_escrow_id(&env);
        let escrow = Escrow {
            id: escrow_id,
            client,
            provider,
            arbitrator,
            token,
            milestones: built_milestones,
            status: EscrowStatus::Created,
            review_period,
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

    /// Cancels a funded escrow, refunding whatever hasn't already been
    /// released or resolved back to the client. Requires both the client's
    /// and the provider's authorization in the same call -- this is how
    /// they mutually agree to unwind an escrow after funding, since neither
    /// can unilaterally cancel once funds are locked (that's what
    /// `cancel_escrow`, above, is restricted to the pre-funding state for).
    ///
    /// Blocked while any milestone is `Disputed`: that milestone already
    /// has a resolution path through the arbitrator, and letting a mutual
    /// cancellation route around it would let either party walk away from
    /// an arbitration they're already in rather than see it through.
    pub fn mutual_cancel_escrow(env: Env, escrow_id: u64) {
        let mut escrow = state::load_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Funded && escrow.status != EscrowStatus::InProgress {
            panic_with_error!(&env, Error::InvalidEscrowStatus);
        }
        for m in escrow.milestones.iter() {
            if m.status == MilestoneStatus::Disputed {
                panic_with_error!(&env, Error::UnresolvedDisputeExists);
            }
        }

        escrow.client.require_auth();
        escrow.provider.require_auth();

        let refund_amount = state::unreleased_amount(&env, &escrow.milestones);

        // Update state before the external token call, same as every other
        // path that moves money: a caller-supplied token contract that
        // calls back in during `transfer` sees this escrow already
        // `Cancelled`, not still `Funded`/`InProgress`.
        escrow.status = EscrowStatus::Cancelled;
        state::save_escrow(&env, &escrow);

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.client,
            &refund_amount,
        );

        EscrowMutuallyCancelled {
            escrow_id,
            refund_amount,
        }
        .publish(&env);
    }

    /// Returns the current state of an escrow.
    pub fn get_escrow(env: Env, escrow_id: u64) -> Escrow {
        state::load_escrow(&env, escrow_id)
    }
}
