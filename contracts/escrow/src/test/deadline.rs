//! Deadline enforcement (`initialize_escrow`, `submit_milestone`) and
//! expiration (`expire_milestone`).

use super::setup;
use crate::types::{MilestoneInput, MilestoneStatus, Resolution};
use soroban_sdk::testutils::{Events, Ledger};
use soroban_sdk::{Env, String, Vec};

fn milestone_with_deadline(env: &Env, deadline: u64) -> Vec<MilestoneInput> {
    let mut v = Vec::new(env);
    v.push_back(MilestoneInput {
        description: String::from_str(env, "Only milestone"),
        amount: 50,
        deadline,
    });
    v
}

#[test]
#[should_panic]
fn test_reject_deadline_in_the_past() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestone_with_deadline(&s.env, 500),
    );
}

#[test]
fn test_deadlines_need_not_be_increasing_across_milestones() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);

    let mut inputs = Vec::new(&s.env);
    inputs.push_back(MilestoneInput {
        description: String::from_str(&s.env, "First milestone"),
        amount: 50,
        deadline: 3_000,
    });
    inputs.push_back(MilestoneInput {
        description: String::from_str(&s.env, "Second milestone"),
        amount: 60,
        deadline: 2_000, // earlier than the first milestone's deadline
    });

    let escrow_id =
        s.contract
            .initialize_escrow(&s.client, &s.provider, &s.arbitrator, &s.token, &inputs);

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.milestones.get(0).unwrap().deadline, 3_000);
    assert_eq!(escrow.milestones.get(1).unwrap().deadline, 2_000);
}

#[test]
#[should_panic]
fn test_reject_deadline_equal_to_now() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestone_with_deadline(&s.env, 1_000),
    );
}

#[test]
fn test_submit_exactly_at_deadline_succeeds() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestone_with_deadline(&s.env, 2_000),
    );
    s.contract.fund_escrow(&escrow_id);

    s.env.ledger().set_timestamp(2_000);
    s.contract.submit_milestone(&escrow_id, &0);

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Submitted
    );
}

#[test]
#[should_panic]
fn test_reject_submission_after_deadline() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestone_with_deadline(&s.env, 2_000),
    );
    s.contract.fund_escrow(&escrow_id);

    s.env.ledger().set_timestamp(2_001);
    s.contract.submit_milestone(&escrow_id, &0);
}

#[test]
fn test_expire_after_deadline_succeeds() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestone_with_deadline(&s.env, 2_000),
    );
    s.contract.fund_escrow(&escrow_id);

    s.env.ledger().set_timestamp(2_001);
    s.contract.expire_milestone(&escrow_id, &0);

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Expired
    );
}

#[test]
#[should_panic]
fn test_reject_expiry_before_deadline_reached() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestone_with_deadline(&s.env, 2_000),
    );
    s.contract.fund_escrow(&escrow_id);

    // Exactly at the deadline: still on time for submission, not yet
    // expirable -- the two checks are complementary, not overlapping.
    s.env.ledger().set_timestamp(2_000);
    s.contract.expire_milestone(&escrow_id, &0);
}

#[test]
#[should_panic]
fn test_reject_expiring_submitted_milestone() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestone_with_deadline(&s.env, 2_000),
    );
    s.contract.fund_escrow(&escrow_id);
    s.contract.submit_milestone(&escrow_id, &0);

    s.env.ledger().set_timestamp(2_001);
    s.contract.expire_milestone(&escrow_id, &0);
}

#[test]
#[should_panic]
fn test_reject_expiring_released_milestone() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestone_with_deadline(&s.env, 2_000),
    );
    s.contract.fund_escrow(&escrow_id);
    s.contract.submit_milestone(&escrow_id, &0);
    s.contract.approve_milestone(&escrow_id, &0);

    s.env.ledger().set_timestamp(2_001);
    s.contract.expire_milestone(&escrow_id, &0);
}

#[test]
#[should_panic]
fn test_reject_expiring_disputed_milestone() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestone_with_deadline(&s.env, 2_000),
    );
    s.contract.fund_escrow(&escrow_id);
    s.env.ledger().set_timestamp(2_001);
    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.client,
        &String::from_str(&s.env, "Provider went silent"),
    );

    s.contract.expire_milestone(&escrow_id, &0);
}

#[test]
#[should_panic]
fn test_reject_expiring_resolved_milestone() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestone_with_deadline(&s.env, 2_000),
    );
    s.contract.fund_escrow(&escrow_id);
    s.env.ledger().set_timestamp(2_001);
    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.client,
        &String::from_str(&s.env, "Provider went silent"),
    );
    s.contract
        .resolve_dispute(&escrow_id, &0, &Resolution::RefundToClient);

    s.contract.expire_milestone(&escrow_id, &0);
}

#[test]
fn test_dispute_can_be_raised_on_expired_milestone() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestone_with_deadline(&s.env, 2_000),
    );
    s.contract.fund_escrow(&escrow_id);
    s.env.ledger().set_timestamp(2_001);
    s.contract.expire_milestone(&escrow_id, &0);

    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.client,
        &String::from_str(&s.env, "Provider never delivered"),
    );

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Disputed
    );
}

#[test]
fn test_expire_milestone_emits_event() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestone_with_deadline(&s.env, 2_000),
    );
    s.contract.fund_escrow(&escrow_id);
    s.env.ledger().set_timestamp(2_001);

    // `events().all()` reflects only the most recent top-level invocation
    // (see `test::events`), so this is `expire_milestone`'s own event count:
    // just `MilestoneExpired` -- no funds move, so no token transfer event.
    s.contract.expire_milestone(&escrow_id, &0);
    assert_eq!(s.env.events().all().events().len(), 1);
}

#[test]
fn test_expire_milestone_is_permissionless() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestone_with_deadline(&s.env, 2_000),
    );
    s.contract.fund_escrow(&escrow_id);
    s.env.ledger().set_timestamp(2_001);

    // No auths mocked at all -- if `expire_milestone` called `require_auth`
    // anywhere, this would panic on a missing auth entry. It doesn't, which
    // is the point: anyone can advance an already-fixed, already-public
    // deadline.
    s.contract.mock_auths(&[]).expire_milestone(&escrow_id, &0);

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Expired
    );
}
