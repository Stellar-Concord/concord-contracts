//! Milestone submission and approval.

use crate::errors::Error;
use crate::events::{
    EscrowCompleted, MilestoneApproved, MilestoneAutoReleased, MilestoneExpired, MilestoneSubmitted,
};
use crate::state;
use crate::types::{EscrowStatus, MilestoneStatus, MAX_URI_LEN};
use crate::{EscrowContract, EscrowContractArgs, EscrowContractClient};
use soroban_sdk::{contractimpl, panic_with_error, token, BytesN, Env, String};

#[contractimpl]
impl EscrowContract {
    /// Provider marks a milestone as delivered, attaching a proof-of
    /// -delivery reference. Authorized by the provider. Rejected once that
    /// milestone's own deadline has passed -- see `expire_milestone` for
    /// what happens to a late milestone instead.
    ///
    /// `evidence_uri` and `evidence_hash` point at off-chain content (e.g.
    /// an IPFS CID or HTTPS URL, and a hash of what's there) rather than
    /// storing the evidence itself on-chain. Required and immutable: once
    /// set here they're never changed again, since this function only ever
    /// runs once per milestone (it requires `Pending`, and nothing moves a
    /// milestone back to `Pending` afterward).
    pub fn submit_milestone(
        env: Env,
        escrow_id: u64,
        milestone_id: u32,
        evidence_uri: String,
        evidence_hash: BytesN<32>,
    ) {
        let mut escrow = state::load_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Funded && escrow.status != EscrowStatus::InProgress {
            panic_with_error!(&env, Error::InvalidEscrowStatus);
        }

        escrow.provider.require_auth();

        let mut milestone = state::get_milestone(&env, &escrow, milestone_id);
        if milestone.status != MilestoneStatus::Pending {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }
        // Inclusive: submitting exactly at the deadline is still on time.
        // `expire_milestone` uses the complementary strict `>` check, so
        // there's no instant where both submission and expiry are valid,
        // and none where neither is.
        if env.ledger().timestamp() > milestone.deadline {
            panic_with_error!(&env, Error::MilestoneDeadlinePassed);
        }
        if evidence_uri.is_empty() || evidence_uri.len() > MAX_URI_LEN {
            panic_with_error!(&env, Error::InvalidUri);
        }
        milestone.status = MilestoneStatus::Submitted;
        milestone.submitted_at = env.ledger().timestamp();
        milestone.evidence_uri = evidence_uri.clone();
        milestone.evidence_hash = evidence_hash.clone();
        escrow.milestones.set(milestone_id, milestone);

        if escrow.status == EscrowStatus::Funded {
            escrow.status = EscrowStatus::InProgress;
        }
        state::save_escrow(&env, &escrow);

        MilestoneSubmitted {
            escrow_id,
            milestone_id,
            evidence_uri,
            evidence_hash,
        }
        .publish(&env);
    }

    /// Marks a `Pending` milestone `Expired` once its deadline has passed.
    /// Moves no funds -- it only records the fact on-chain, which then lets
    /// the client raise a dispute to recover the locked amount through
    /// arbitration (or, once mutual cancellation ships, agree with the
    /// provider to just walk away).
    ///
    /// Deliberately permissionless (no `require_auth`): the only thing this
    /// function can do is flip a status flag once an already-fixed,
    /// already-public deadline has passed. There's no discretion to
    /// exploit -- the caller can't make it fire early (the timestamp check
    /// is absolute) and doesn't influence what it does. Restricting it to
    /// one party would only add friction (someone has to remember to call
    /// it); anyone -- a keeper, the platform's own backend, either party --
    /// can advance it once eligible.
    pub fn expire_milestone(env: Env, escrow_id: u64, milestone_id: u32) {
        let mut escrow = state::load_escrow(&env, escrow_id);

        let mut milestone = state::get_milestone(&env, &escrow, milestone_id);
        if milestone.status != MilestoneStatus::Pending {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }
        if env.ledger().timestamp() <= milestone.deadline {
            panic_with_error!(&env, Error::DeadlineNotReached);
        }

        milestone.status = MilestoneStatus::Expired;
        escrow.milestones.set(milestone_id, milestone);
        state::save_escrow(&env, &escrow);

        MilestoneExpired {
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

    /// Pays out a `Submitted` milestone to the provider once the escrow's
    /// `review_period` has elapsed since submission without the client
    /// approving it (or raising a dispute). Same effect as
    /// `approve_milestone` -- funds move, milestone ends up `Released` --
    /// just triggered by a timeout instead of the client's signature.
    ///
    /// Deliberately permissionless, for the same reason as
    /// `expire_milestone`: the recipient and amount both come from stored
    /// escrow state, never from the caller, so there's nothing a caller can
    /// redirect by calling this. All they can do is trigger, at the
    /// earliest once the client's own agreed-to review window has passed,
    /// the same payout the client could have triggered immediately by
    /// approving. Anyone -- a keeper, the platform's backend, the provider
    /// themselves -- can call it once eligible; a client who wants to
    /// actually review has that entire window to call `approve_milestone`
    /// or `raise_dispute` first.
    pub fn auto_release_milestone(env: Env, escrow_id: u64, milestone_id: u32) {
        let mut escrow = state::load_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::InProgress {
            panic_with_error!(&env, Error::InvalidEscrowStatus);
        }

        let mut milestone = state::get_milestone(&env, &escrow, milestone_id);
        if milestone.status != MilestoneStatus::Submitted {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }
        // Saturating, not checked: unlike the money math in `state.rs`, an
        // overflow here isn't an error condition to reject -- it just means
        // the review period is absurdly long, so the correct eligible time
        // is "unreachable", which `u64::MAX` already represents.
        let eligible_at = milestone.submitted_at.saturating_add(escrow.review_period);
        if env.ledger().timestamp() <= eligible_at {
            panic_with_error!(&env, Error::ReviewPeriodNotElapsed);
        }

        // Same checks-effects-interactions ordering as `approve_milestone`:
        // state is `Released` before the external token call, so a
        // reentrant call sees it already settled instead of paying twice.
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

        MilestoneAutoReleased {
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
