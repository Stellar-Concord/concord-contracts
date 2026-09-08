//! Timeout protection: `auto_release_milestone` pays the provider once a
//! `Submitted` milestone's review period has elapsed without the client
//! approving it or raising a dispute.

use super::{milestones, setup, TestSetup};
use crate::types::{EscrowStatus, MilestoneStatus, Resolution};
use soroban_sdk::testutils::{Events, Ledger};
use soroban_sdk::String;

const REVIEW_PERIOD: u64 = 500;

fn setup_submitted_milestone(s: &TestSetup) -> u64 {
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &REVIEW_PERIOD,
    );
    s.contract.fund_escrow(&escrow_id);
    s.contract.submit_milestone(&escrow_id, &0);
    escrow_id
}

#[test]
#[should_panic]
fn test_reject_auto_release_before_review_period_elapsed() {
    let s = setup();
    let escrow_id = setup_submitted_milestone(&s);

    // Exactly at the boundary: the period has not yet strictly elapsed.
    let submitted_at = s.env.ledger().timestamp();
    s.env.ledger().set_timestamp(submitted_at + REVIEW_PERIOD);
    s.contract.auto_release_milestone(&escrow_id, &0);
}

#[test]
fn test_auto_release_succeeds_after_review_period_elapsed() {
    let s = setup();
    let escrow_id = setup_submitted_milestone(&s);

    let submitted_at = s.env.ledger().timestamp();
    s.env
        .ledger()
        .set_timestamp(submitted_at + REVIEW_PERIOD + 1);
    s.contract.auto_release_milestone(&escrow_id, &0);

    assert_eq!(s.token_client.balance(&s.provider), 100);
    assert_eq!(s.token_client.balance(&s.contract.address), 0);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Released
    );
    assert_eq!(escrow.status, EscrowStatus::Completed);
}

#[test]
#[should_panic]
fn test_reject_auto_release_on_pending_milestone() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &REVIEW_PERIOD,
    );
    s.contract.fund_escrow(&escrow_id);
    // Never submitted -- there's no `submitted_at` for the review period to
    // count from.

    let now = s.env.ledger().timestamp();
    s.env.ledger().set_timestamp(now + REVIEW_PERIOD + 1);
    s.contract.auto_release_milestone(&escrow_id, &0);
}

#[test]
#[should_panic]
fn test_reject_auto_release_on_already_released_milestone() {
    let s = setup();
    let escrow_id = setup_submitted_milestone(&s);
    s.contract.approve_milestone(&escrow_id, &0);

    let now = s.env.ledger().timestamp();
    s.env.ledger().set_timestamp(now + REVIEW_PERIOD + 1);
    s.contract.auto_release_milestone(&escrow_id, &0);
}

#[test]
#[should_panic]
fn test_reject_auto_release_on_disputed_milestone() {
    let s = setup();
    let escrow_id = setup_submitted_milestone(&s);
    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.client,
        &String::from_str(&s.env, "Not what was scoped"),
    );

    let now = s.env.ledger().timestamp();
    s.env.ledger().set_timestamp(now + REVIEW_PERIOD + 1);
    s.contract.auto_release_milestone(&escrow_id, &0);
}

#[test]
#[should_panic]
fn test_reject_auto_release_on_resolved_milestone() {
    let s = setup();
    let escrow_id = setup_submitted_milestone(&s);
    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.client,
        &String::from_str(&s.env, "Not what was scoped"),
    );
    s.contract
        .resolve_dispute(&escrow_id, &0, &Resolution::ReleaseToProvider);

    let now = s.env.ledger().timestamp();
    s.env.ledger().set_timestamp(now + REVIEW_PERIOD + 1);
    s.contract.auto_release_milestone(&escrow_id, &0);
}

#[test]
fn test_auto_release_leaves_other_milestones_unaffected() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(
            &s.env,
            &[("Milestone A", 100i128), ("Milestone B", 200i128)],
        ),
        &REVIEW_PERIOD,
    );
    s.contract.fund_escrow(&escrow_id);
    s.contract.submit_milestone(&escrow_id, &0);

    let submitted_at = s.env.ledger().timestamp();
    s.env
        .ledger()
        .set_timestamp(submitted_at + REVIEW_PERIOD + 1);
    s.contract.auto_release_milestone(&escrow_id, &0);

    assert_eq!(s.token_client.balance(&s.provider), 100);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::InProgress);
    assert_eq!(
        escrow.milestones.get(1).unwrap().status,
        MilestoneStatus::Pending
    );

    // The untouched milestone still proceeds normally afterward.
    s.contract.submit_milestone(&escrow_id, &1);
    s.contract.approve_milestone(&escrow_id, &1);
    assert_eq!(s.token_client.balance(&s.provider), 300);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Completed);
}

#[test]
fn test_client_can_still_approve_during_review_period() {
    let s = setup();
    let escrow_id = setup_submitted_milestone(&s);

    // Still well within the review window -- auto-release isn't the only
    // path to payment, just the fallback one.
    s.contract.approve_milestone(&escrow_id, &0);
    assert_eq!(s.token_client.balance(&s.provider), 100);
}

#[test]
fn test_auto_release_emits_event() {
    let s = setup();
    let escrow_id = setup_submitted_milestone(&s);

    let now = s.env.ledger().timestamp();
    s.env.ledger().set_timestamp(now + REVIEW_PERIOD + 1);

    // token transfer + MilestoneAutoReleased + EscrowCompleted (this was
    // the only milestone), same shape as `approve_milestone`'s event count.
    s.contract.auto_release_milestone(&escrow_id, &0);
    assert_eq!(s.env.events().all().events().len(), 3);
}

#[test]
fn test_auto_release_is_permissionless() {
    let s = setup();
    let escrow_id = setup_submitted_milestone(&s);

    let now = s.env.ledger().timestamp();
    s.env.ledger().set_timestamp(now + REVIEW_PERIOD + 1);

    // No auths mocked at all -- if `auto_release_milestone` called
    // `require_auth` on anyone but the contract itself (for its own
    // outgoing transfer, which needs none), this would panic on a missing
    // auth entry.
    s.contract
        .mock_auths(&[])
        .auto_release_milestone(&escrow_id, &0);

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Released
    );
}
