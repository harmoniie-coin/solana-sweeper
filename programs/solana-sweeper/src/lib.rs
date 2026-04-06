use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer, CloseAccount};
use std::str::FromStr;

declare_id!("G68yXrhYqYojo18AFkY4oaBcUqogZXz52C48imEJ6W5s");

#[account]
pub struct GlobalState {
    pub authorized_bot: Pubkey,
    pub vault: Pubkey,
    pub initialized: bool,
}

#[program]
pub mod solana_sweeper {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, authorized_bot: Pubkey) -> Result<()> {
        // Check if already initialized
        if ctx.accounts.global_state.initialized {
            return Err(CustomError::AlreadyInitialized.into());
        }

        // Set authorized bot, vault and mark as initialized
        ctx.accounts.global_state.authorized_bot = authorized_bot;
        ctx.accounts.global_state.vault = ctx.accounts.vault.key();
        ctx.accounts.global_state.initialized = true;

        Ok(())
    }

    pub fn batch_sweep(ctx: Context<BatchSweep>, user_ids: Vec<String>, bumps: Vec<u8>) -> Result<()> {

        // Verify global state is initialized
        if !ctx.accounts.global_state.initialized {
            return Err(CustomError::NotInitialized.into());
        }

        // Verify bot has permission (check against global state)
        if ctx.accounts.bot.key != &ctx.accounts.global_state.authorized_bot {
           return Err(CustomError::Unauthorized.into());
        }

        // Verify vault matches the initialized vault
        if ctx.accounts.vault.key() != ctx.accounts.global_state.vault {
            return Err(CustomError::InvalidVault.into());
        }

        // Validate inputs - now we expect 2x accounts (token account + PDA for each user)
        if user_ids.is_empty() || user_ids.len() * 2 != ctx.remaining_accounts.len() || user_ids.len() != bumps.len() {
            return Err(CustomError::InvalidInput.into());
        }


        // Process each user
        for i in 0..user_ids.len() {
            // Get user token account and PDA account
            let user_token_account = &ctx.remaining_accounts[i * 2];
            let pda_account = &ctx.remaining_accounts[i * 2 + 1];
            
            // Get user token account data
            let amount = {
                let account_data = match user_token_account.data.try_borrow() {
                    Ok(data) => data,
                    Err(_) => continue,
                };
                
                // Check if account has enough data (TokenAccount is 165 bytes)
                if account_data.len() < 165 {
                    continue;
                }
                
                // Get amount from token account (offset 64, 8 bytes for u64)
                u64::from_le_bytes(account_data[64..72].try_into().unwrap())
            };
            
            // Skip if no tokens
            if amount == 0 {
                continue;
            }

            // Prepare PDA seeds
            let id_str = &user_ids[i];
            let id_bytes = id_str.as_bytes();
            let seeds = &[
                b"user_deposit",
                id_bytes,
                &[bumps[i]],
            ];

            // Create seeds array
            let seeds_array = &[&seeds[..]];

            // Transfer tokens using PDA as authority
            unsafe {
                let from = std::mem::transmute(user_token_account.to_account_info());
                let to = std::mem::transmute(ctx.accounts.vault.to_account_info());
                let authority = std::mem::transmute(pda_account.to_account_info());
                let token_program = std::mem::transmute(ctx.accounts.token_program.to_account_info());
                
                token::transfer(
                    CpiContext::new_with_signer(
                        token_program,
                        Transfer {
                            from,
                            to,
                            authority,
                        },
                        seeds_array,
                    ),
                    amount
                )?;
            }


        }

        Ok(())
    }

    pub fn batch_close_ata(ctx: Context<CloseAta>, user_ids: Vec<String>, bumps: Vec<u8>) -> Result<()> {
        // Verify global state is initialized
        if !ctx.accounts.global_state.initialized {
            return Err(CustomError::NotInitialized.into());
        }

        // Verify bot has permission (check against global state)
        if ctx.accounts.bot.key != &ctx.accounts.global_state.authorized_bot {
            return Err(CustomError::Unauthorized.into());
        }

        // Validate inputs - now we expect 2x accounts (token account + PDA for each user)
        if user_ids.is_empty() || user_ids.len() * 2 != ctx.remaining_accounts.len() || user_ids.len() != bumps.len() {
            return Err(CustomError::InvalidInput.into());
        }

        // Process each user
        for i in 0..user_ids.len() {
            // Get user token account and PDA account
            let user_token_account = &ctx.remaining_accounts[i * 2];
            let pda_account = &ctx.remaining_accounts[i * 2 + 1];

            // Prepare PDA seeds
            let id_str = &user_ids[i];
            let id_bytes = id_str.as_bytes();
            let seeds = &[
                b"user_deposit",
                id_bytes,
                &[bumps[i]],
            ];

            // Create seeds array
            let seeds_array = &[&seeds[..]];

            // Close ATA using PDA as authority
            unsafe {
                let account = std::mem::transmute(user_token_account.to_account_info());
                let authority = std::mem::transmute(pda_account.to_account_info());
                let recipient = std::mem::transmute(ctx.accounts.bot.to_account_info());
                let token_program = std::mem::transmute(ctx.accounts.token_program.to_account_info());
                
                token::close_account(
                    CpiContext::new_with_signer(
                        token_program,
                        CloseAccount {
                            account,
                            destination: recipient,
                            authority,
                        },
                        seeds_array,
                    )
                )?;
            }
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init, 
        payer = admin, 
        space = 8 + 32 + 32 + 1,
        seeds = [b"global_state", vault.key.as_ref()],
        bump
    )]
    pub global_state: Account<'info, GlobalState>,
    
    /// CHECK: This is a vault account
    #[account(mut)]
    pub vault: AccountInfo<'info>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BatchSweep<'info> {
    #[account(mut)]
    pub bot: Signer<'info>,

    /// CHECK: This is a token account
    #[account(mut)]
    pub vault: AccountInfo<'info>,

    #[account(
        seeds = [b"global_state", vault.key.as_ref()],
        bump
    )]
    pub global_state: Account<'info, GlobalState>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CloseAta<'info> {
    #[account(mut)]
    pub bot: Signer<'info>,

    /// CHECK: This is a vault account
    pub vault: AccountInfo<'info>,

    #[account(
        seeds = [b"global_state", vault.key.as_ref()],
        bump
    )]
    pub global_state: Account<'info, GlobalState>,
    pub token_program: Program<'info, Token>,
}

#[error_code]
pub enum CustomError {
    #[msg("Invalid input parameters")]
    InvalidInput,
    #[msg("Unauthorized access")]
    Unauthorized,
    #[msg("Already initialized")]
    AlreadyInitialized,
    #[msg("Not initialized")]
    NotInitialized,
    #[msg("Invalid vault")]
    InvalidVault,
}
