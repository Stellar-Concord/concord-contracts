use super::setup;
use crate::types::{EscrowStatus, MilestoneStatus};
use soroban_sdk::{vec, String};

#[test]
fn test_full_escrow_lifecycle() {
    let s = setup();

    let descriptions = vec![
        &s.env,
        String::from_str(&s.env, "Design mockups"),
        String::from_str(&s.env, "Implementation"),
    ];
    let amounts = vec![&s.env, 100i128, 300i128];

    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &descriptions,
        &amounts,
    );

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Created);
    assert_eq!(escrow.milestones.len(), 2);

    s.contract.fund_escrow(&escrow_id);
    assert_eq!(s.token_client.balance(&s.client), 1_000_000 - 400);
    assert_eq!(s.token_client.balance(&s.contract.address), 400);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Funded);

    s.contract.submit_milestone(&escrow_id, &0);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::InProgress);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Submitted
    );

    s.contract.approve_milestone(&escrow_id, &0);
    assert_eq!(s.token_client.balance(&s.provider), 100);
    assert_eq!(s.token_client.balance(&s.contract.address), 300);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::InProgress);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Released
    );

    s.contract.submit_milestone(&escrow_id, &1);
    s.contract.approve_milestone(&escrow_id, &1);

    assert_eq!(s.token_client.balance(&s.provider), 400);
    assert_eq!(s.token_client.balance(&s.contract.address), 0);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Completed);
}

#[test]
fn test_cancel_before_funding() {
    let s = setup();
    let descriptions = vec![&s.env, String::from_str(&s.env, "Only milestone")];
    let amounts = vec![&s.env, 50i128];
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &descriptions,
        &amounts,
    );

    s.contract.cancel_escrow(&escrow_id);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);
}

#[test]
#[should_panic]
fn test_cannot_fund_twice() {
    let s = setup();
    let descriptions = vec![&s.env, String::from_str(&s.env, "Only milestone")];
    let amounts = vec![&s.env, 50i128];
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &descriptions,
        &amounts,
    );

    s.contract.fund_escrow(&escrow_id);
    s.contract.fund_escrow(&escrow_id);
}

#[test]
#[should_panic]
fn test_cannot_cancel_after_funding() {
    let s = setup();
    let descriptions = vec![&s.env, String::from_str(&s.env, "Only milestone")];
    let amounts = vec![&s.env, 50i128];
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &descriptions,
        &amounts,
    );

    s.contract.fund_escrow(&escrow_id);
    s.contract.cancel_escrow(&escrow_id);
}

#[test]
#[should_panic]
fn test_no_milestones_rejected() {
    let s = setup();
    let descriptions = vec![&s.env];
    let amounts = vec![&s.env];
    s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &descriptions,
        &amounts,
    );
}

#[test]
#[should_panic]
fn test_client_cannot_also_be_arbitrator() {
    let s = setup();
    let descriptions = vec![&s.env, String::from_str(&s.env, "Only milestone")];
    let amounts = vec![&s.env, 50i128];
    s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.client,
        &s.token,
        &descriptions,
        &amounts,
    );
}

#[test]
#[should_panic]
fn test_provider_cannot_also_be_arbitrator() {
    let s = setup();
    let descriptions = vec![&s.env, String::from_str(&s.env, "Only milestone")];
    let amounts = vec![&s.env, 50i128];
    s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.provider,
        &s.token,
        &descriptions,
        &amounts,
    );
}

#[test]
#[should_panic]
fn test_client_cannot_also_be_provider() {
    let s = setup();
    let descriptions = vec![&s.env, String::from_str(&s.env, "Only milestone")];
    let amounts = vec![&s.env, 50i128];
    s.contract.initialize_escrow(
        &s.client,
        &s.client,
        &s.arbitrator,
        &s.token,
        &descriptions,
        &amounts,
    );
}
