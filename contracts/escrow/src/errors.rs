use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    EscrowNotFound = 1,
    MilestoneNotFound = 2,
    InvalidEscrowStatus = 5,
    NoMilestones = 6,
    // 7 was MismatchedMilestoneInputs, for a two-parallel-Vec milestone
    // input shape. Removed (not reused) when milestone creation moved to
    // one Vec<MilestoneInput> per milestone, which makes a length
    // mismatch between description/amount/deadline structurally
    // impossible instead of a runtime check.
    InvalidMilestoneAmount = 8,
    InvalidMilestoneStatus = 9,
    AmountOverflow = 10,
    DisputeNotFound = 11,
    NotDisputeParty = 12,
    InvalidSplitPercentage = 13,
    RolesMustBeDistinct = 14,
    /// A milestone's deadline was not strictly in the future at creation.
    InvalidDeadline = 15,
    /// `submit_milestone` called after that milestone's own deadline.
    MilestoneDeadlinePassed = 16,
    /// `expire_milestone` called before the deadline has passed.
    DeadlineNotReached = 17,
}
