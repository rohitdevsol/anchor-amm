use anchor_lang::prelude::*;
mod state;
mod instructions;
mod errors;

use state::*;
use instructions::*;
use errors::*;
declare_id!("8mDQUcdAxwvqhrLUtiKFqjzNyG9e5PsWVtnNoGpZXBoZ");

#[program]
pub mod anchor_amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
