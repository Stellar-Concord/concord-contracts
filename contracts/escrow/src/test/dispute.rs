use super::{milestones, setup, TestSetup, DEFAULT_REVIEW_PERIOD};
use crate::types::{EscrowStatus, MilestoneStatus, Resolution};
use soroban_sdk::String;

fn setup_single_disputed_milestone(s: &TestSetup, amount: i128) -> u64 {
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", amount)]),
        &DEFAULT_REVIEW_PERIOD,
    );
    s.contract.fund_escrow(&escrow_id);
    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.provider,
        &String::from_str(&s.env, "Client is unresponsive"),
    );
    escrow_id
}

#[test]
fn test_dispute_release_to_provider() {
    let s = setup();
    let escrow_id = setup_single_disputed_milestone(&s, 100);

    let dispute = s.contract.get_dispute(&escrow_id, &0);
    assert_eq!(dispute.raised_by, s.provider);

    s.contract
        .resolve_dispute(&escrow_id, &0, &Resolution::ReleaseToProvider);

    assert_eq!(s.token_client.balance(&s.provider), 100);
    assert_eq!(s.token_client.balance(&s.contract.address), 0);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Resolved
    );
    assert_eq!(escrow.status, EscrowStatus::Completed);
    assert_eq!(
        s.contract.get_dispute_resolution(&escrow_id, &0),
        Resolution::ReleaseToProvider
    );
}

#[test]
fn test_dispute_refund_to_client() {
    let s = setup();
    let escrow_id = setup_single_disputed_milestone(&s, 100);

    s.contract
        .resolve_dispute(&escrow_id, &0, &Resolution::RefundToClient);

    assert_eq!(s.token_client.balance(&s.client), 1_000_000);
    assert_eq!(s.token_client.balance(&s.provider), 0);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Completed);
}

#[test]
fn test_dispute_split() {
    let s = setup();
    let escrow_id = setup_single_disputed_milestone(&s, 100);

    s.contract
        .resolve_dispute(&escrow_id, &0, &Resolution::Split(6_000));

    assert_eq!(s.token_client.balance(&s.provider), 60);
    assert_eq!(s.token_client.balance(&s.client), 1_000_000 - 100 + 40);
    assert_eq!(s.token_client.balance(&s.contract.address), 0);
}

#[test]
fn test_dispute_leaves_other_milestones_unaffected() {
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
        &DEFAULT_REVIEW_PERIOD,
    );
    s.contract.fund_escrow(&escrow_id);

    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.client,
        &String::from_str(&s.env, "Not delivered as scoped"),
    );

    super::submit(&s, escrow_id, 1);
    s.contract.approve_milestone(&escrow_id, &1);
    assert_eq!(s.token_client.balance(&s.provider), 200);

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::InProgress);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Disputed
    );
}

#[test]
#[should_panic]
fn test_dispute_by_non_party_rejected() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
    );
    s.contract.fund_escrow(&escrow_id);

    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.arbitrator,
        &String::from_str(&s.env, "Not my dispute to raise"),
    );
}

#[test]
#[should_panic]
fn test_cannot_dispute_released_milestone() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
    );
    s.contract.fund_escrow(&escrow_id);
    super::submit(&s, escrow_id, 0);
    s.contract.approve_milestone(&escrow_id, &0);

    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.client,
        &String::from_str(&s.env, "Too late"),
    );
}

#[test]
#[should_panic]
fn test_cannot_resolve_undisputed_milestone() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
    );
    s.contract.fund_escrow(&escrow_id);

    s.contract
        .resolve_dispute(&escrow_id, &0, &Resolution::ReleaseToProvider);
}

#[test]
#[should_panic]
fn test_split_rejects_invalid_bps() {
    let s = setup();
    let escrow_id = setup_single_disputed_milestone(&s, 100);

    s.contract
        .resolve_dispute(&escrow_id, &0, &Resolution::Split(10_001));
}
