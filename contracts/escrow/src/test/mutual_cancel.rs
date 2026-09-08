//! Mutual cancellation after funding: `mutual_cancel_escrow` requires both
//! the client's and the provider's authorization in the same call, and
//! refunds only whatever hasn't already been released or resolved.

use super::{milestones, setup, DEFAULT_REVIEW_PERIOD};
use crate::types::{EscrowStatus, MilestoneInput, Resolution};
use soroban_sdk::testutils::{Events, Ledger, MockAuth, MockAuthInvoke};
use soroban_sdk::{IntoVal, String, Vec};

#[test]
#[should_panic]
fn test_mutual_cancel_requires_both_signatures_missing_provider() {
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

    // Only the client signs; the contract also requires the provider's.
    s.contract
        .mock_auths(&[MockAuth {
            address: &s.client,
            invoke: &MockAuthInvoke {
                contract: &s.contract.address,
                fn_name: "mutual_cancel_escrow",
                args: (escrow_id,).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .mutual_cancel_escrow(&escrow_id);
}

#[test]
#[should_panic]
fn test_mutual_cancel_requires_both_signatures_missing_client() {
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

    // Only the provider signs; the contract also requires the client's.
    s.contract
        .mock_auths(&[MockAuth {
            address: &s.provider,
            invoke: &MockAuthInvoke {
                contract: &s.contract.address,
                fn_name: "mutual_cancel_escrow",
                args: (escrow_id,).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .mutual_cancel_escrow(&escrow_id);
}

#[test]
#[should_panic]
fn test_mutual_cancel_rejected_before_funding() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
    );

    s.contract.mutual_cancel_escrow(&escrow_id);
}

#[test]
#[should_panic]
fn test_mutual_cancel_rejected_after_completion() {
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
    s.contract.submit_milestone(&escrow_id, &0);
    s.contract.approve_milestone(&escrow_id, &0);

    s.contract.mutual_cancel_escrow(&escrow_id);
}

#[test]
fn test_mutual_cancel_refunds_full_amount_when_untouched() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("A", 100i128), ("B", 200i128)]),
        &DEFAULT_REVIEW_PERIOD,
    );
    s.contract.fund_escrow(&escrow_id);
    let client_balance_before = s.token_client.balance(&s.client);

    s.contract.mutual_cancel_escrow(&escrow_id);

    assert_eq!(
        s.token_client.balance(&s.client),
        client_balance_before + 300
    );
    assert_eq!(s.token_client.balance(&s.contract.address), 0);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);
}

#[test]
fn test_mutual_cancel_refunds_only_unreleased_portion() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("A", 100i128), ("B", 200i128)]),
        &DEFAULT_REVIEW_PERIOD,
    );
    s.contract.fund_escrow(&escrow_id);
    s.contract.submit_milestone(&escrow_id, &0);
    s.contract.approve_milestone(&escrow_id, &0);

    let client_balance_before = s.token_client.balance(&s.client);
    s.contract.mutual_cancel_escrow(&escrow_id);

    // A's 100 already went to the provider; only B's 200 comes back.
    assert_eq!(
        s.token_client.balance(&s.client),
        client_balance_before + 200
    );
    assert_eq!(s.token_client.balance(&s.provider), 100);
    assert_eq!(s.token_client.balance(&s.contract.address), 0);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);
}

#[test]
fn test_mutual_cancel_includes_expired_milestone() {
    let s = setup();
    s.env.ledger().set_timestamp(1_000);
    let mut inputs = Vec::new(&s.env);
    inputs.push_back(MilestoneInput {
        description: String::from_str(&s.env, "Only milestone"),
        amount: 100,
        deadline: 2_000,
    });
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &inputs,
        &DEFAULT_REVIEW_PERIOD,
    );
    s.contract.fund_escrow(&escrow_id);
    s.env.ledger().set_timestamp(2_001);
    s.contract.expire_milestone(&escrow_id, &0);

    let client_balance_before = s.token_client.balance(&s.client);
    s.contract.mutual_cancel_escrow(&escrow_id);

    assert_eq!(
        s.token_client.balance(&s.client),
        client_balance_before + 100
    );
}

#[test]
#[should_panic]
fn test_mutual_cancel_blocked_by_open_dispute() {
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
        &s.client,
        &String::from_str(&s.env, "Not delivered"),
    );

    s.contract.mutual_cancel_escrow(&escrow_id);
}

#[test]
fn test_mutual_cancel_allowed_after_dispute_resolved() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("A", 100i128), ("B", 200i128)]),
        &DEFAULT_REVIEW_PERIOD,
    );
    s.contract.fund_escrow(&escrow_id);
    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.client,
        &String::from_str(&s.env, "Not delivered"),
    );
    s.contract
        .resolve_dispute(&escrow_id, &0, &Resolution::RefundToClient);

    // Milestone A is now `Resolved` (already refunded by the dispute), B is
    // still `Pending` -- the resolved dispute no longer blocks cancellation.
    let client_balance_before = s.token_client.balance(&s.client);
    s.contract.mutual_cancel_escrow(&escrow_id);

    assert_eq!(
        s.token_client.balance(&s.client),
        client_balance_before + 200
    );
}

#[test]
fn test_mutual_cancel_emits_event() {
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

    // token transfer + EscrowMutuallyCancelled.
    s.contract.mutual_cancel_escrow(&escrow_id);
    assert_eq!(s.env.events().all().events().len(), 2);
}

#[test]
#[should_panic]
fn test_cannot_submit_after_mutual_cancellation() {
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
    s.contract.mutual_cancel_escrow(&escrow_id);

    s.contract.submit_milestone(&escrow_id, &0);
}
