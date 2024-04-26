use std::{ops::Deref, sync::Arc};

use anchor_client::{Client, Cluster, Program};
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use token_faucet::{program::TokenFaucet as TokenFaucetProgram, ID as TOKEN_FAUCET_PROGRAM_ID};

use crate::{RpcAccountProvider, Wallet};

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

    // pub async fn initialize(&self) -> Transaction {
    //     let pubkey = self.get_faucet_config_public_key();
    //     self.
    // }

    // pub async fn create_associated_token_account_and_mint_to_instructions(
    // ) -> (Pubkey, Instruction, Instruction) {
    // }
}
