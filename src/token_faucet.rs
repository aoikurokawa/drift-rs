use std::{ops::Deref, sync::Arc};

use anchor_client::{Client, Cluster, Program};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    rent::Rent,
    signature::{Keypair, Signature},
    signer::Signer,
    system_program, sysvar,
    transaction::Transaction,
};
use token_faucet::{
    accounts::InitializeFaucet, program::TokenFaucet as TokenFaucetProgram,
    ID as TOKEN_FAUCET_PROGRAM_ID,
};

use crate::{
    types::{SdkError, SdkResult},
    RpcAccountProvider, Wallet,
};

pub struct TokenFaucet {
    // connection:Connection
    wallet: Wallet,
    program: Program<Arc<Keypair>>,
    provider: RpcAccountProvider,
    mint: Pubkey,
    // opts: Option<ConfirmOptions>
}

impl TokenFaucet {
    pub fn new(wallet: Wallet, mint: Pubkey) -> Self {
        let provider = RpcAccountProvider::new("");
        let signer = wallet.signer;
        let client = Client::new(Cluster::Devnet, signer);
        let program = client.program(TOKEN_FAUCET_PROGRAM_ID);

        Self {
            wallet,
            program,
            provider,
            mint,
        }
    }

    pub fn get_faucet_config_public_key_and_nonce(&self) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[&b"faucet_config"[..], &self.mint.to_bytes()],
            &TOKEN_FAUCET_PROGRAM_ID,
        )
    }

    pub fn get_mint_authority(&self) -> Pubkey {
        Pubkey::find_program_address(
            &[&b"mint_authority"[..], &self.mint.to_bytes()],
            &TOKEN_FAUCET_PROGRAM_ID,
        )
        .0
    }

    pub fn get_faucet_config_public_key(&self) -> Pubkey {
        self.get_faucet_config_public_key_and_nonce().0
    }

    pub async fn initialize(&self) -> SdkResult<Signature> {
        let pubkey = self.get_faucet_config_public_key();
        let my_account_kp = Keypair::new();
        self.program
            .request()
            .accounts(InitializeFaucet {
                faucet_config: pubkey,
                admin: self.wallet.signer(),
                mint_account: self.mint,
                rent: sysvar::rent::id(),
                system_program: system_program::id(),
                token_program: anchor_spl::token::ID,
            })
            .signer(&my_account_kp)
            .send()
            .map_err(|e| SdkError::AnchorClient(e))
    }

    // pub async fn create_associated_token_account_and_mint_to_instructions(
    // ) -> (Pubkey, Instruction, Instruction) {
    // }
}
