use anchor_spl::token::spl_token::{instruction, state::Mint};
use drift_sdk::{constants::TOKEN_PROGRAM_ID, DriftClient, RpcAccountProvider};
use solana_sdk::{
    program_pack::Pack, rent::Rent, signature::Keypair, signer::Signer, system_instruction,
    sysvar::Sysvar,
};

pub async fn mock_usdc_mint(client: &DriftClient<RpcAccountProvider>) -> Keypair {
    let fake_usdc_mint = Keypair::new();
    let rent = Rent::get().expect("get rent");
    let mint_len = Mint::LEN;
    let create_usdc_mint_account_ix = system_instruction::create_account(
        &client.wallet().authority(),
        &fake_usdc_mint.pubkey(),
        rent.minimum_balance(mint_len),
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
    let message = client
        .init_tx(&client.wallet().default_sub_account(), false)
        .await
        .expect("init tx")
        .add_ixs(ixs)
        .build();
    // fakeUSDCTx.add(createUSDCMintAccountIx);
    // fakeUSDCTx.add(initCollateralMintIx);
    let _ = client
        .sign_and_send(message)
        .await
        .expect("sign and send message");

    fake_usdc_mint
}
