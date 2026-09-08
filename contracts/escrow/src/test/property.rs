//! Property tests for the settlement math in `state.rs`. Unlike the
//! example-based tests elsewhere, these check invariants across a wide
//! range of generated inputs rather than a handful of hand-picked cases.

use crate::state;
use crate::types::{Milestone, MilestoneStatus};
use proptest::prelude::*;
use soroban_sdk::{Env, String, Vec};

/// Comfortably covers any realistic token amount (even at 18 decimals with
/// a huge total supply) while staying far enough under `i128::MAX / 10_000`
/// that `apply_bps` can't overflow -- that boundary is covered separately
/// by `apply_bps_overflow_is_caught_not_wrapped` below.
const MAX_REALISTIC_AMOUNT: i128 = 1_000_000_000_000_000_000_000;

proptest! {
    /// `apply_bps` must compute exactly `amount * bps / 10_000`. This is
    /// the one property here that actually pins down the formula itself --
    /// the others (sum-to-total, bounded, monotonic) all still hold even if
    /// the divisor were wrong, so on their own they wouldn't catch e.g. a
    /// `/ 10_001` typo. This would.
    #[test]
    fn prop_apply_bps_matches_reference_formula(
        amount in 0i128..=MAX_REALISTIC_AMOUNT,
        bps in 0u32..=10_000,
    ) {
        let env = Env::default();
        let actual = state::apply_bps(&env, amount, bps);
        let expected = (amount * bps as i128) / 10_000;
        prop_assert_eq!(actual, expected);
    }

    /// The provider's share plus the remainder must always equal the whole
    /// amount, for any valid split percentage.
    #[test]
    fn prop_split_amounts_sum_to_total(
        amount in 0i128..=MAX_REALISTIC_AMOUNT,
        bps in 0u32..=10_000,
    ) {
        let env = Env::default();
        let provider_amount = state::apply_bps(&env, amount, bps);
        let client_amount = amount - provider_amount;
        prop_assert_eq!(provider_amount + client_amount, amount);
    }

    /// The provider's share can never exceed the total, nor go negative,
    /// for any valid split percentage.
    #[test]
    fn prop_provider_share_is_bounded(
        amount in 0i128..=MAX_REALISTIC_AMOUNT,
        bps in 0u32..=10_000,
    ) {
        let env = Env::default();
        let provider_amount = state::apply_bps(&env, amount, bps);
        prop_assert!(provider_amount >= 0);
        prop_assert!(provider_amount <= amount);
    }

    /// A higher basis-point split can never give the provider a *smaller*
    /// share of the same amount.
    #[test]
    fn prop_provider_share_is_monotonic_in_bps(
        amount in 0i128..=MAX_REALISTIC_AMOUNT,
        low_bps in 0u32..=10_000,
        high_bps in 0u32..=10_000,
    ) {
        let (low_bps, high_bps) = if low_bps <= high_bps {
            (low_bps, high_bps)
        } else {
            (high_bps, low_bps)
        };
        let env = Env::default();
        let low_share = state::apply_bps(&env, amount, low_bps);
        let high_share = state::apply_bps(&env, amount, high_bps);
        prop_assert!(low_share <= high_share);
    }

    /// `total_amount`'s checked-addition loop must agree with plain i128
    /// summation whenever that summation doesn't itself overflow.
    #[test]
    fn prop_total_amount_matches_plain_sum(
        amounts in proptest::collection::vec(1i128..=MAX_REALISTIC_AMOUNT, 0..10),
    ) {
        let env = Env::default();
        let expected: i128 = amounts.iter().sum();

        let mut milestones = Vec::new(&env);
        for (i, amount) in amounts.iter().enumerate() {
            milestones.push_back(Milestone {
                id: i as u32,
                description: String::from_str(&env, "milestone"),
                amount: *amount,
                status: MilestoneStatus::Pending,
                deadline: 0,
            });
        }

        let actual = state::total_amount(&env, &milestones);
        prop_assert_eq!(actual, expected);
    }
}

#[test]
#[should_panic]
fn apply_bps_overflow_is_caught_not_wrapped() {
    let env = Env::default();
    // amount * bps overflows i128 here; this must panic via the checked
    // multiplication in `apply_bps`, not silently wrap to a bogus result.
    state::apply_bps(&env, i128::MAX, 10_000);
}

#[test]
#[should_panic]
fn total_amount_overflow_is_caught_not_wrapped() {
    let env = Env::default();
    let mut milestones = Vec::new(&env);
    for _ in 0..2 {
        milestones.push_back(Milestone {
            id: 0,
            description: String::from_str(&env, "milestone"),
            amount: i128::MAX,
            status: MilestoneStatus::Pending,
            deadline: 0,
        });
    }
    state::total_amount(&env, &milestones);
}
