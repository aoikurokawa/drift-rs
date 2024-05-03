use std::sync::Arc;

use anchor_client::{Client, Cluster, Program};
use anchor_spl::associated_token::get_associated_token_address;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    system_program, sysvar,
};
use spl_associated_token_account::instruction::create_associated_token_account;
use token_faucet::{
    accounts::{self, InitializeFaucet},
    instruction, FaucetConfig, ID as TOKEN_FAUCET_PROGRAM_ID,
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
        let signer = wallet.clone().signer;
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

    pub fn fetch_state(&self) -> SdkResult<FaucetConfig> {
        self.program
            .account(self.get_faucet_config_public_key())
            .map_err(|e| SdkError::AnchorClient(e))
    }

    fn mint_to_user_ix(
        &self,
        user_token_account: &Pubkey,
        amount: u64,
    ) -> SdkResult<Vec<Instruction>> {
        let signer = &self.wallet.signer;
        let signer = Keypair::from_bytes(&signer.to_bytes())
            .map_err(|e| SdkError::Generic(e.to_string()))?;
        self.program
            .request()
            .accounts(accounts::MintToUser {
                faucet_config: self.get_faucet_config_public_key(),
                mint_account: self.mint,
                user_token_account: *user_token_account,
                mint_authority: self.get_mint_authority(),
                token_program: anchor_spl::token::ID,
            })
            .args(instruction::MintToUser { amount })
            .signer(&signer)
            .instructions()
            .map_err(|e| SdkError::AnchorClient(e))
    }

    pub async fn mint_to_user(
        &self,
        user_token_account: &Pubkey,
        amount: u64,
    ) -> SdkResult<Signature> {
        let signer = &self.wallet.signer;
        let signer = Keypair::from_bytes(&signer.to_bytes())
            .map_err(|e| SdkError::Generic(e.to_string()))?;
        let mint_ix = self.mint_to_user_ix(user_token_account, amount)?;
        match mint_ix.get(0) {
            Some(ix) => self
                .program
                .request()
                .instruction(ix.clone())
                .signer(&signer)
                .send()
                .map_err(|e| SdkError::AnchorClient(e)),
            None => Err(SdkError::Generic(
                "fail to get mint instruction".to_string(),
            )),
        }
    }

    pub async fn create_associated_token_account_and_mint_to_instructions(
        &self,
        user_pubkey: Pubkey,
        amount: u64,
    ) -> SdkResult<(Pubkey, Instruction, Instruction)> {
        let state = self.fetch_state()?;
        let associated_token_pubkey = self.get_assosciated_mock_usd_mint_address(user_pubkey)?;
        let associated_token_account_ix = create_associated_token_account(
            &self.wallet.signer(),
            &associated_token_pubkey,
            &user_pubkey,
            &state.mint,
        );
        let mint_to_ix = self.mint_to_user_ix(&associated_token_pubkey, amount)?;

        Ok((
            associated_token_pubkey,
            associated_token_account_ix,
            mint_to_ix[0].clone(),
        ))
    }

    pub fn get_assosciated_mock_usd_mint_address(&self, user_pubkey: Pubkey) -> SdkResult<Pubkey> {
        let state = self.fetch_state()?;

        Ok(get_associated_token_address(&state.mint, &user_pubkey))
    }
}
