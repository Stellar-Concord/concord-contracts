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
    /// The deadline passed while still `Pending` (nobody submitted in
    /// time). Set only by `expire_milestone`; never set automatically —
    /// see that function's docs. Still disputable, so the client can
    /// recover the locked amount through arbitration instead of waiting
    /// on a mutual cancellation.
    Expired,
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

/// One milestone as supplied to `initialize_escrow`. Bundling description,
/// amount, and deadline per-milestone (rather than three parallel `Vec`s)
/// makes a length mismatch between them structurally impossible, instead
/// of something that has to be checked and rejected at runtime.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneInput {
    pub description: String,
    pub amount: i128,
    /// Ledger timestamp (seconds). Must be strictly in the future at
    /// creation time. Milestones are independent in this contract already
    /// (nothing sequences submitting/approving one before another), so
    /// deadlines are independent too — not required to be increasing
    /// across a milestone list.
    pub deadline: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub id: u32,
    pub description: String,
    pub amount: i128,
    pub status: MilestoneStatus,
    pub deadline: u64,
    /// Ledger timestamp `submit_milestone` was called at. `0` until then --
    /// a real submission at the Unix epoch is not a case any deployed
    /// network needs to represent, so it doubles as "not yet submitted"
    /// without needing an `Option`.
    pub submitted_at: u64,
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
    /// Seconds the client has, after a milestone is submitted, before
    /// `auto_release_milestone` becomes callable on it. Set once at
    /// creation and applied to every milestone in this escrow -- see
    /// `initialize_escrow`'s docs for why this lives here rather than as a
    /// contract-wide constant or a per-milestone field.
    pub review_period: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    EscrowCounter,
    Escrow(u64),
    Dispute(u64, u32),
    DisputeResolution(u64, u32),
}
