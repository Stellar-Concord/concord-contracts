use crate::types::{Milestone, Resolution};
use soroban_sdk::{contractevent, Address, String, Vec};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowCreated {
    #[topic]
    pub escrow_id: u64,
    pub client: Address,
    pub provider: Address,
    pub arbitrator: Address,
    pub token: Address,
    pub milestones: Vec<Milestone>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowFunded {
    #[topic]
    pub escrow_id: u64,
    pub total: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneSubmitted {
    #[topic]
    pub escrow_id: u64,
    pub milestone_id: u32,
}

/// Emitted by `expire_milestone` once a `Pending` milestone's deadline has
/// passed. No funds move -- this only marks the fact on-chain.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneExpired {
    #[topic]
    pub escrow_id: u64,
    pub milestone_id: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneApproved {
    #[topic]
    pub escrow_id: u64,
    pub milestone_id: u32,
    pub amount: i128,
}

/// Emitted by `auto_release_milestone` instead of `MilestoneApproved` --
/// same effect on the milestone (paid out, status `Released`), but this
/// name lets off-chain consumers tell a timeout payout apart from the
/// client actually approving the work.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneAutoReleased {
    #[topic]
    pub escrow_id: u64,
    pub milestone_id: u32,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowCancelled {
    #[topic]
    pub escrow_id: u64,
}

/// Emitted by `mutual_cancel_escrow`. Distinct from `EscrowCancelled`
/// (the pre-funding, client-only cancellation, which moves no money) since
/// this one always refunds `refund_amount` back to the client.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowMutuallyCancelled {
    #[topic]
    pub escrow_id: u64,
    pub refund_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowCompleted {
    #[topic]
    pub escrow_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeRaised {
    #[topic]
    pub escrow_id: u64,
    pub milestone_id: u32,
    pub raised_by: Address,
    pub reason: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeResolved {
    #[topic]
    pub escrow_id: u64,
    pub milestone_id: u32,
    pub resolution: Resolution,
}
