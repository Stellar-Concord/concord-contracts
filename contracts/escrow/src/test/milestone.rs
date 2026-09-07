use super::setup;
use soroban_sdk::{vec, String};

#[test]
#[should_panic]
fn test_cannot_approve_without_submit() {
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
    s.contract.approve_milestone(&escrow_id, &0);
}

#[test]
#[should_panic]
fn test_milestone_not_found() {
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
    s.contract.submit_milestone(&escrow_id, &5);
}
