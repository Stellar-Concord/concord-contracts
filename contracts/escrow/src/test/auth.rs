//! `setup()` uses `env.mock_all_auths()`, which makes *any* address's
//! signature succeed -- useful for exercising business logic, but it never
//! proves `require_auth()` is actually gating access. These tests instead
//! scope authorization to a single, specific, wrong address per call, so
//! they fail unless the real signer check is what's rejecting them.

use super::{milestones, setup, DEFAULT_REVIEW_PERIOD};
use soroban_sdk::testutils::{MockAuth, MockAuthInvoke};
use soroban_sdk::{IntoVal, String};

#[test]
#[should_panic]
fn test_initialize_escrow_requires_client_signature() {
    let s = setup();
    let milestone_input = milestones(&s.env, &[("Only milestone", 50i128)]);

    // Only the provider's signature is mocked; the contract requires the
    // client's.
    s.contract
        .mock_auths(&[MockAuth {
            address: &s.provider,
            invoke: &MockAuthInvoke {
                contract: &s.contract.address,
                fn_name: "initialize_escrow",
                args: (
                    &s.client,
                    &s.provider,
                    &s.arbitrator,
                    &s.token,
                    &milestone_input,
                    DEFAULT_REVIEW_PERIOD,
                )
                    .into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .initialize_escrow(
            &s.client,
            &s.provider,
            &s.arbitrator,
            &s.token,
            &milestone_input,
            &DEFAULT_REVIEW_PERIOD,
        );
}

#[test]
#[should_panic]
fn test_fund_escrow_requires_client_signature() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 50i128)]),
        &DEFAULT_REVIEW_PERIOD,
    );

    s.contract
        .mock_auths(&[MockAuth {
            address: &s.provider,
            invoke: &MockAuthInvoke {
                contract: &s.contract.address,
                fn_name: "fund_escrow",
                args: (escrow_id,).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .fund_escrow(&escrow_id);
}

#[test]
#[should_panic]
fn test_submit_milestone_requires_provider_signature() {
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

    // The client, not the provider, signs this attempt.
    s.contract
        .mock_auths(&[MockAuth {
            address: &s.client,
            invoke: &MockAuthInvoke {
                contract: &s.contract.address,
                fn_name: "submit_milestone",
                args: (escrow_id, 0u32).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .submit_milestone(&escrow_id, &0);
}

#[test]
#[should_panic]
fn test_approve_milestone_requires_client_signature() {
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
    s.contract.submit_milestone(&escrow_id, &0);

    // The provider signs, hoping to approve (and get paid for) their own
    // milestone.
    s.contract
        .mock_auths(&[MockAuth {
            address: &s.provider,
            invoke: &MockAuthInvoke {
                contract: &s.contract.address,
                fn_name: "approve_milestone",
                args: (escrow_id, 0u32).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .approve_milestone(&escrow_id, &0);
}

#[test]
#[should_panic]
fn test_raise_dispute_requires_raised_by_to_actually_sign() {
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
    let reason = String::from_str(&s.env, "Spoofed dispute");

    // Claims `raised_by: client`, but only the provider actually signs --
    // impersonating the client this way must not work.
    s.contract
        .mock_auths(&[MockAuth {
            address: &s.provider,
            invoke: &MockAuthInvoke {
                contract: &s.contract.address,
                fn_name: "raise_dispute",
                args: (escrow_id, 0u32, &s.client, &reason).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .raise_dispute(&escrow_id, &0, &s.client, &reason);
}

#[test]
#[should_panic]
fn test_resolve_dispute_requires_arbitrator_signature() {
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
    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.client,
        &String::from_str(&s.env, "Not delivered"),
    );

    use crate::types::Resolution;
    let resolution = Resolution::ReleaseToProvider;

    // The client signs, hoping to resolve their own dispute in their favor.
    s.contract
        .mock_auths(&[MockAuth {
            address: &s.client,
            invoke: &MockAuthInvoke {
                contract: &s.contract.address,
                fn_name: "resolve_dispute",
                args: (escrow_id, 0u32, &resolution).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .resolve_dispute(&escrow_id, &0, &resolution);
}
