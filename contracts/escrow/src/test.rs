use crate::types::{EscrowStatus, MilestoneStatus, Resolution};
use crate::{EscrowContract, EscrowContractClient};
use soroban_sdk::{testutils::Address as _, token, vec, Address, Env, String};

fn create_token<'a>(
    env: &Env,
    admin: &Address,
) -> (
    Address,
    token::TokenClient<'a>,
    token::StellarAssetClient<'a>,
) {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let address = sac.address();
    (
        address.clone(),
        token::TokenClient::new(env, &address),
        token::StellarAssetClient::new(env, &address),
    )
}

struct TestSetup {
    env: Env,
    client: Address,
    provider: Address,
    arbitrator: Address,
    token: Address,
    token_client: token::TokenClient<'static>,
    contract: EscrowContractClient<'static>,
}

fn setup() -> TestSetup {
    let env = Env::default();
    env.mock_all_auths();

    let client = Address::generate(&env);
    let provider = Address::generate(&env);
    let arbitrator = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token, token_client, token_admin_client) = create_token(&env, &token_admin);
    token_admin_client.mint(&client, &1_000_000);

    let contract_id = env.register(EscrowContract, ());
    let contract = EscrowContractClient::new(&env, &contract_id);

    TestSetup {
        env,
        client,
        provider,
        arbitrator,
        token,
        token_client,
        contract,
    }
}

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

fn setup_single_disputed_milestone(s: &TestSetup, amount: i128) -> u64 {
    let descriptions = vec![&s.env, String::from_str(&s.env, "Only milestone")];
    let amounts = vec![&s.env, amount];
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &descriptions,
        &amounts,
    );
    s.contract.fund_escrow(&escrow_id);
    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.provider,
        &String::from_str(&s.env, "Client is unresponsive"),
    );
    escrow_id
}

#[test]
fn test_dispute_release_to_provider() {
    let s = setup();
    let escrow_id = setup_single_disputed_milestone(&s, 100);

    let dispute = s.contract.get_dispute(&escrow_id, &0);
    assert_eq!(dispute.raised_by, s.provider);

    s.contract
        .resolve_dispute(&escrow_id, &0, &Resolution::ReleaseToProvider);

    assert_eq!(s.token_client.balance(&s.provider), 100);
    assert_eq!(s.token_client.balance(&s.contract.address), 0);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Resolved
    );
    assert_eq!(escrow.status, EscrowStatus::Completed);
    assert_eq!(
        s.contract.get_dispute_resolution(&escrow_id, &0),
        Resolution::ReleaseToProvider
    );
}

#[test]
fn test_dispute_refund_to_client() {
    let s = setup();
    let escrow_id = setup_single_disputed_milestone(&s, 100);

    s.contract
        .resolve_dispute(&escrow_id, &0, &Resolution::RefundToClient);

    assert_eq!(s.token_client.balance(&s.client), 1_000_000);
    assert_eq!(s.token_client.balance(&s.provider), 0);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Completed);
}

#[test]
fn test_dispute_split() {
    let s = setup();
    let escrow_id = setup_single_disputed_milestone(&s, 100);

    s.contract
        .resolve_dispute(&escrow_id, &0, &Resolution::Split(6_000));

    assert_eq!(s.token_client.balance(&s.provider), 60);
    assert_eq!(s.token_client.balance(&s.client), 1_000_000 - 100 + 40);
    assert_eq!(s.token_client.balance(&s.contract.address), 0);
}

#[test]
fn test_dispute_leaves_other_milestones_unaffected() {
    let s = setup();
    let descriptions = vec![
        &s.env,
        String::from_str(&s.env, "Milestone A"),
        String::from_str(&s.env, "Milestone B"),
    ];
    let amounts = vec![&s.env, 100i128, 200i128];
    let escrow_id = s.contract.initialize_escrow(
        &s.client,
        &s.provider,
        &s.arbitrator,
        &s.token,
        &descriptions,
        &amounts,
    );
    s.contract.fund_escrow(&escrow_id);

    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.client,
        &String::from_str(&s.env, "Not delivered as scoped"),
    );

    s.contract.submit_milestone(&escrow_id, &1);
    s.contract.approve_milestone(&escrow_id, &1);
    assert_eq!(s.token_client.balance(&s.provider), 200);

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::InProgress);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Disputed
    );
}

#[test]
#[should_panic]
fn test_dispute_by_non_party_rejected() {
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
    s.contract.fund_escrow(&escrow_id);

    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.arbitrator,
        &String::from_str(&s.env, "Not my dispute to raise"),
    );
}

#[test]
#[should_panic]
fn test_cannot_dispute_released_milestone() {
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
    s.contract.fund_escrow(&escrow_id);
    s.contract.submit_milestone(&escrow_id, &0);
    s.contract.approve_milestone(&escrow_id, &0);

    s.contract.raise_dispute(
        &escrow_id,
        &0,
        &s.client,
        &String::from_str(&s.env, "Too late"),
    );
}

#[test]
#[should_panic]
fn test_cannot_resolve_undisputed_milestone() {
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
    s.contract.fund_escrow(&escrow_id);

    s.contract
        .resolve_dispute(&escrow_id, &0, &Resolution::ReleaseToProvider);
}

#[test]
#[should_panic]
fn test_split_rejects_invalid_bps() {
    let s = setup();
    let escrow_id = setup_single_disputed_milestone(&s, 100);

    s.contract
        .resolve_dispute(&escrow_id, &0, &Resolution::Split(10_001));
}

#[test]
fn test_events_are_published_on_state_transitions() {
    use soroban_sdk::testutils::Events;

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
