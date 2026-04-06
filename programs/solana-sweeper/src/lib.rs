use anchor_lang::prelude::*;

declare_id!("En1eftfS9FC422pDccuwWwSVwpuaoapw5PUAEWuL4Hkt");

#[program]
pub mod solana_sweeper {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
