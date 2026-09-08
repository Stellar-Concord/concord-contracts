use super::{milestones, setup, DEFAULT_REVIEW_PERIOD};
use soroban_sdk::testutils::Events;

#[test]
fn test_events_are_published_on_state_transitions() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
    );
    assert_eq!(s.env.events().all().events().len(), 1);

    // fund_escrow also triggers the token contract's own "transfer" event.
    s.contract.fund_escrow(&escrow_id);
    assert_eq!(s.env.events().all().events().len(), 2);

    s.contract.submit_milestone(&escrow_id, &0);
    assert_eq!(s.env.events().all().events().len(), 1);

    // approve_milestone triggers a token "transfer" event, MilestoneApproved,
    // and EscrowCompleted, since this was the only milestone.
    s.contract.approve_milestone(&escrow_id, &0);
    assert_eq!(s.env.events().all().events().len(), 3);
}
