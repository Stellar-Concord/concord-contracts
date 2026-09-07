use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    EscrowNotFound = 1,
    MilestoneNotFound = 2,
    NotClient = 3,
    NotProvider = 4,
    InvalidEscrowStatus = 5,
    NoMilestones = 6,
    MismatchedMilestoneInputs = 7,
    InvalidMilestoneAmount = 8,
    InvalidMilestoneStatus = 9,
    AmountOverflow = 10,
}
