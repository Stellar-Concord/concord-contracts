use super::setup;
use soroban_sdk::{testutils::Events, vec, String};

#[test]
fn test_events_are_published_on_state_transitions() {
    let s = setup();
    let descriptions = vec![&s.env, String::from_str(&s.env, "Only milestone")];
    let amounts = vec![&s.env, 100i128];
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &descriptions,
        &amounts,
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
