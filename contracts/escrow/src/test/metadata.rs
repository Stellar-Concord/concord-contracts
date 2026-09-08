//! Escrow metadata: an optional title + off-chain description URI/hash,
//! set once at creation and never changed again.

use super::{milestones, no_metadata, setup, DEFAULT_REVIEW_PERIOD};
use crate::types::{EscrowMetadata, MAX_TITLE_LEN, MAX_URI_LEN};
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{BytesN, String};

#[test]
fn test_create_escrow_without_metadata() {
    let s = setup();
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
        &no_metadata(&s.env),
    );

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.title, String::from_str(&s.env, ""));
    assert_eq!(escrow.metadata_uri, String::from_str(&s.env, ""));
    assert_eq!(escrow.metadata_hash, BytesN::from_array(&s.env, &[0u8; 32]));
}

#[test]
fn test_create_escrow_with_metadata() {
    let s = setup();
    let title = String::from_str(&s.env, "Website redesign");
    let metadata_uri = String::from_str(&s.env, "ipfs://QmExampleMetadata");
    let metadata_hash = BytesN::from_array(&s.env, &[5u8; 32]);
    let metadata = EscrowMetadata {
        title: title.clone(),
        metadata_uri: metadata_uri.clone(),
        metadata_hash: metadata_hash.clone(),
    };

    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
        &metadata,
    );

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.title, title);
    assert_eq!(escrow.metadata_uri, metadata_uri);
    assert_eq!(escrow.metadata_hash, metadata_hash);
}

#[test]
fn test_created_at_matches_ledger_timestamp_at_creation() {
    let s = setup();
    s.env.ledger().set_timestamp(12_345);

    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
        &no_metadata(&s.env),
    );

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.created_at, 12_345);
}

#[test]
#[should_panic]
fn test_reject_title_too_long() {
    let s = setup();
    let too_long: std::string::String = "a".repeat((MAX_TITLE_LEN + 1) as usize);
    let metadata = EscrowMetadata {
        title: String::from_str(&s.env, &too_long),
        metadata_uri: String::from_str(&s.env, ""),
        metadata_hash: BytesN::from_array(&s.env, &[0u8; 32]),
    };

    s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
        &metadata,
    );
}

#[test]
fn test_title_at_max_length_accepted() {
    let s = setup();
    let exactly_max: std::string::String = "a".repeat(MAX_TITLE_LEN as usize);
    let metadata = EscrowMetadata {
        title: String::from_str(&s.env, &exactly_max),
        metadata_uri: String::from_str(&s.env, ""),
        metadata_hash: BytesN::from_array(&s.env, &[0u8; 32]),
    };

    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
        &metadata,
    );

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.title.len(), MAX_TITLE_LEN);
}

#[test]
#[should_panic]
fn test_reject_metadata_uri_too_long() {
    let s = setup();
    let too_long: std::string::String = "a".repeat((MAX_URI_LEN + 1) as usize);
    let metadata = EscrowMetadata {
        title: String::from_str(&s.env, ""),
        metadata_uri: String::from_str(&s.env, &too_long),
        metadata_hash: BytesN::from_array(&s.env, &[0u8; 32]),
    };

    s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
        &metadata,
    );
}

#[test]
fn test_metadata_uri_at_max_length_accepted() {
    let s = setup();
    let exactly_max: std::string::String = "a".repeat(MAX_URI_LEN as usize);
    let metadata = EscrowMetadata {
        title: String::from_str(&s.env, ""),
        metadata_uri: String::from_str(&s.env, &exactly_max),
        metadata_hash: BytesN::from_array(&s.env, &[0u8; 32]),
    };

    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
        &metadata,
    );

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.metadata_uri.len(), MAX_URI_LEN);
}

#[test]
fn test_metadata_survives_full_lifecycle() {
    let s = setup();
    let title = String::from_str(&s.env, "Logo design");
    let metadata = EscrowMetadata {
        title: title.clone(),
        metadata_uri: String::from_str(&s.env, "ipfs://QmBrief"),
        metadata_hash: BytesN::from_array(&s.env, &[3u8; 32]),
    };

    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &milestones(&s.env, &[("Only milestone", 100i128)]),
        &DEFAULT_REVIEW_PERIOD,
        &metadata,
    );
    s.contract.fund_escrow(&escrow_id);
    super::submit(&s, escrow_id, 0);
    s.contract.approve_milestone(&escrow_id, &0);

    // Immutable: still there, unchanged, after the escrow completes.
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.title, title);
}
