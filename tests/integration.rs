use anchor_spl::token::spl_token::{instruction, state::Mint};
use drift::math::constants::{BASE_PRECISION_I64, LAMPORTS_PER_SOL_I64, PRICE_PRECISION_U64};
use drift_sdk::{
    constants::TOKEN_PROGRAM_ID,
    get_market_accounts,
    token_faucet::TokenFaucet,
    types::{Context, MarketId, NewOrder, VersionedMessage},
    DriftClient, RpcAccountProvider, Wallet,
};
use solana_sdk::{
    message::v0, program_pack::Pack, signature::Keypair, signer::Signer, system_instruction,
};

/// keypair for integration tests
fn test_keypair() -> Keypair {
    // let mut private_key = std::env::var("TEST_PRIVATE_KEY").expect("TEST_PRIVATE_KEY set");
    // let private_key = "".to_string();
    // if private_key.is_empty() {
    //     Keypair::new()
    // } else {
    // Keypair::from_base58_string(private_key.as_str())
    // }
    Keypair::new()
}

async fn mock_usdc_mint(client: &DriftClient<RpcAccountProvider>) -> Keypair {
    let fake_usdc_mint = Keypair::new();
    // let rent = Rent::get().expect("get rent");
    let mint_len = Mint::LEN;
    let create_usdc_mint_account_ix = system_instruction::create_account(
        &client.wallet().authority(),
        &fake_usdc_mint.pubkey(),
        u64::MAX,
        mint_len as u64,
        &TOKEN_PROGRAM_ID,
    );
    let init_collateral_mint_ix = instruction::initialize_mint(
        &anchor_spl::token::ID,
        &fake_usdc_mint.pubkey(),
        client.wallet().authority(),
        None,
        6,
    )
    .expect("initialize usdc mint");

    let ixs = vec![create_usdc_mint_account_ix, init_collateral_mint_ix];
    // let fake_usdt_tx = Transaction::new_signed_with_payer(
    //     &[create_usdc_mint_account_ix, init_collateral_mint_ix],
    //     Some(wallet.authority()),
    //     &[&client.wallet().into()],
    //     client
    //         .get_latest_blockhash()
    //         .await
    //         .expect("get latest blockhash"),
    // );
    // let message = client
    //     .init_tx(&client.wallet().default_sub_account(), false)
    //     .await
    //     .expect("init tx")
    //     .add_ixs(ixs)
    //     .build();
    let message = {
        let lookup_tables = vec![client.program_data().lookup_table.clone()];
        let message = v0::Message::try_compile(
            &client.wallet().authority(),
            ixs.as_slice(),
            lookup_tables.as_slice(),
            Default::default(),
        )
        .expect("ok");
        VersionedMessage::V0(message)
    };
    let _ = client
        .sign_and_send_with_signers(message, vec![client.wallet().into(), fake_usdc_mint])
        .await
        .expect("sign and send message");

    fake_usdc_mint
}

#[tokio::test]
async fn get_oracle_prices() {
    let client = DriftClient::new(
        Context::DevNet,
        RpcAccountProvider::new("https://api.devnet.solana.com"),
        Keypair::new().into(),
    )
    .await
    .expect("connects");
    let price = client.oracle_price(MarketId::perp(0)).await.expect("ok");
    assert!(price > 0);
    dbg!(price);
    let price = client.oracle_price(MarketId::spot(1)).await.expect("ok");
    assert!(price > 0);
    dbg!(price);
}

#[tokio::test]
async fn get_market_accounts_works() {
    let client = DriftClient::new(
        Context::DevNet,
        RpcAccountProvider::new("https://api.devnet.solana.com"),
        Keypair::new().into(),
    )
    .await
    .expect("connects");

    let (spot, perp) = get_market_accounts(client.inner()).await.unwrap();
    assert!(spot.len() > 1);
    assert!(perp.len() > 1);
}

#[tokio::test]
async fn place_and_cancel_orders() {
    let wallet: Wallet = test_keypair().into();
    let mut client = DriftClient::new(
        Context::DevNet,
        RpcAccountProvider::new("https://api.devnet.solana.com"),
        wallet.clone(),
    )
    .await
    .expect("connects");
    client
        .add_user(client.active_sub_account_id)
        .await
        .expect("add user");

    let sol_perp = client.market_lookup("sol-perp").expect("exists");
    let sol_spot = client.market_lookup("sol").expect("exists");

    let tx = client
        .init_tx(&wallet.default_sub_account(), false)
        .await
        .unwrap()
        .cancel_all_orders()
        .place_orders(vec![
            NewOrder::limit(sol_perp)
                .amount(1 * BASE_PRECISION_I64)
                .price(40 * PRICE_PRECISION_U64)
                .post_only(drift_sdk::types::PostOnlyParam::MustPostOnly)
                .build(),
            NewOrder::limit(sol_spot)
                .amount(-1 * LAMPORTS_PER_SOL_I64)
                .price(400 * PRICE_PRECISION_U64)
                .post_only(drift_sdk::types::PostOnlyParam::MustPostOnly)
                .build(),
        ])
        .cancel_all_orders()
        .build();

    dbg!(tx.clone());

    let result = client.sign_and_send_with_wallet(tx).await;
    dbg!(&result);
    assert!(result.is_ok());
}

#[tokio::test]
async fn place_and_take() {
    let wallet: Wallet = test_keypair().into();
    let market_index = 0;
    let mut client = DriftClient::new(
        Context::DevNet,
        RpcAccountProvider::new("https://api.devnet.solana.com"),
        wallet.clone(),
    )
    .await
    .expect("connects");
    let mock_usdc = mock_usdc_mint(&client).await;
    let token_faucet = TokenFaucet::new(wallet.clone(), mock_usdc.pubkey());

    match client.get_user_account(wallet.authority()).await {
        Ok(user) => {
            eprintln!("add user account");
            client
                .add_user(user.sub_account_id)
                .await
                .expect("add user");
        }
        Err(_) => {
            eprintln!("creating new user account");
            let _ = client
                .initialize_user_account_for_devnet(market_index, token_faucet, 10)
                .await
                .expect("initialize user account for devnet");
        }
    }

    let sol_perp = client.market_lookup("sol-perp").expect("exists");

    let order = NewOrder::limit(sol_perp)
        .amount(1 * BASE_PRECISION_I64)
        .price(40 * PRICE_PRECISION_U64)
        .build();
    let tx = client
        .init_tx(&wallet.default_sub_account(), false)
        .await
        .unwrap()
        .place_and_take(order, None, None, None)
        .build();

    let result = client.sign_and_send_with_wallet(tx).await;
    dbg!(&result);
    // TODO: add a place and make to match against
    assert!(result.is_err());
}
