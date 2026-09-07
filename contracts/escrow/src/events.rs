use crate::types::Resolution;
use soroban_sdk::{contractevent, Address, String};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowCreated {
    #[topic]
    pub escrow_id: u64,
    pub client: Address,
    pub provider: Address,
    pub arbitrator: Address,
    pub token: Address,
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

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneApproved {
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
