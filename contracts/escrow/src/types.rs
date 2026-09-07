use soroban_sdk::{contracttype, Address, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Created,
    Funded,
    InProgress,
    Completed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MilestoneStatus {
    Pending,
    Submitted,
    Released,
    Disputed,
    Resolved,
}

/// How a disputed milestone's funds are settled by the arbitrator.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Resolution {
    ReleaseToProvider,
    RefundToClient,
    /// Splits the milestone amount by `provider_bps` basis points
    /// (0..=10000) to the provider; the remainder goes to the client.
    Split(u32),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dispute {
    pub milestone_id: u32,
    pub raised_by: Address,
    pub reason: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub id: u32,
    pub description: String,
    pub amount: i128,
    pub status: MilestoneStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub id: u64,
    pub client: Address,
    pub provider: Address,
    pub arbitrator: Address,
    pub token: Address,
    pub milestones: Vec<Milestone>,
    pub status: EscrowStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    EscrowCounter,
    Escrow(u64),
    Dispute(u64, u32),
    DisputeResolution(u64, u32),
}
