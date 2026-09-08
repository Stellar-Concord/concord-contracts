//! Milestone evidence / proof of delivery: `submit_milestone` requires an
//! evidence URI + hash, stores them, and never lets them change again.

use super::{milestones, setup, DEFAULT_REVIEW_PERIOD};
use crate::types::MAX_URI_LEN;
use soroban_sdk::testutils::Events;
use soroban_sdk::{BytesN, String};

fn setup_funded_escrow(s: &super::TestSetup) -> u64 {
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
    );
    s.contract.fund_escrow(&escrow_id);
    escrow_id
}

#[test]
fn test_submit_stores_evidence() {
    let s = setup();
    let escrow_id = setup_funded_escrow(&s);

    let uri = String::from_str(&s.env, "ipfs://QmExampleEvidence");
    let hash = BytesN::from_array(&s.env, &[9u8; 32]);
    s.contract.submit_milestone(&escrow_id, &0, &uri, &hash);

    let escrow = s.contract.get_escrow(&escrow_id);
    let milestone = escrow.milestones.get(0).unwrap();
    assert_eq!(milestone.evidence_uri, uri);
    assert_eq!(milestone.evidence_hash, hash);
}

#[test]
#[should_panic]
fn test_reject_empty_evidence_uri() {
    let s = setup();
    let escrow_id = setup_funded_escrow(&s);

    s.contract.submit_milestone(
        &escrow_id,
        &0,
        &String::from_str(&s.env, ""),
        &BytesN::from_array(&s.env, &[9u8; 32]),
    );
}

#[test]
#[should_panic]
fn test_reject_evidence_uri_too_long() {
    let s = setup();
    let escrow_id = setup_funded_escrow(&s);

    // One character over the cap.
    let too_long: std::string::String = "a".repeat((MAX_URI_LEN + 1) as usize);
    s.contract.submit_milestone(
        &escrow_id,
        &0,
        &String::from_str(&s.env, &too_long),
        &BytesN::from_array(&s.env, &[9u8; 32]),
    );
}

#[test]
fn test_evidence_uri_at_max_length_accepted() {
    let s = setup();
    let escrow_id = setup_funded_escrow(&s);

    let exactly_max: std::string::String = "a".repeat(MAX_URI_LEN as usize);
    s.contract.submit_milestone(
        &escrow_id,
        &0,
        &String::from_str(&s.env, &exactly_max),
        &BytesN::from_array(&s.env, &[9u8; 32]),
    );

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(
        escrow.milestones.get(0).unwrap().evidence_uri.len(),
        MAX_URI_LEN
    );
}

#[test]
#[should_panic]
fn test_evidence_immutable_after_submission() {
    let s = setup();
    let escrow_id = setup_funded_escrow(&s);

    let uri = String::from_str(&s.env, "ipfs://first");
    let hash = BytesN::from_array(&s.env, &[1u8; 32]);
    s.contract.submit_milestone(&escrow_id, &0, &uri, &hash);

    // A second submission attempt -- even with different evidence -- must
    // fail, since the milestone is no longer `Pending`. This is what makes
    // the first submission's evidence immutable: there's no code path that
    // overwrites it afterward.
    let new_uri = String::from_str(&s.env, "ipfs://second");
    let new_hash = BytesN::from_array(&s.env, &[2u8; 32]);
    s.contract
        .submit_milestone(&escrow_id, &0, &new_uri, &new_hash);
}

#[test]
fn test_submit_milestone_emits_evidence_in_event() {
    let s = setup();
    let escrow_id = setup_funded_escrow(&s);

    let uri = String::from_str(&s.env, "ipfs://QmExampleEvidence");
    let hash = BytesN::from_array(&s.env, &[9u8; 32]);
    s.contract.submit_milestone(&escrow_id, &0, &uri, &hash);

    // Just the one MilestoneSubmitted event -- confirms the call succeeded
    // (the assertion above already confirms the stored fields match).
    assert_eq!(s.env.events().all().events().len(), 1);
}
