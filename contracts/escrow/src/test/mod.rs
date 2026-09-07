//! Shared test fixtures. Cases are split by domain, mirroring
//! `escrow.rs` / `milestone.rs` / `dispute.rs`.

mod auth;
mod dispute;
mod escrow;
mod events;
mod milestone;

use crate::{EscrowContract, EscrowContractClient};
use soroban_sdk::{testutils::Address as _, token, Address, Env};

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
