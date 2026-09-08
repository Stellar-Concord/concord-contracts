use super::{milestones, setup, DEFAULT_REVIEW_PERIOD};

#[test]
#[should_panic]
fn test_cannot_approve_without_submit() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 50i128)]),
        &DEFAULT_REVIEW_PERIOD,
    );

    s.contract.fund_escrow(&escrow_id);
    s.contract.approve_milestone(&escrow_id, &0);
}

#[test]
#[should_panic]
fn test_milestone_not_found() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 50i128)]),
        &DEFAULT_REVIEW_PERIOD,
    );

    s.contract.fund_escrow(&escrow_id);
    s.contract.submit_milestone(&escrow_id, &5);
}
