use anchor_lang::prelude::*;
use anchor_spl::{ token::{ Mint, Token, TokenAccount } };

use crate::{ state::Config };

#[derive(Accounts)]
#[instruction(seed:u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_x: Account<'info, Mint>,

    #[account(mint::token_program = token_program)]
    pub mint_y: Account<'info, Mint>,

    #[account(
        init, // Create new token account
        payer = user, // Admin pays for creation
        associated_token::mint = mint_x, // Associated with mint_x
        associated_token::authority = config, // Owned by config PDA
        associated_token::token_program = token_program // Uses SPL Token program
    )]
    pub vault_x: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = user,
        associated_token::mint = mint_y,
        associated_token::authority = config,
        associated_token::token_program = token_program
    )]
    pub vault_y: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = user,
        mint::decimals = 6,
        mint::authority = config.key(),
        seeds = [b"lp", config.key().as_ref()],
        bump
    )]
    pub mint_lp: Account<'info, Mint>,

    #[account(
        init,
        payer = user,
        seeds = [b"config", seed.to_le_bytes().as_ref()],
        bump,
        space = Config::INIT_SPACE
    )]
    pub config: Account<'info, Config>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, Token>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(
        &mut self,
        seed: u64,
        fee: u16,
        authority: Option<Pubkey>,
        bumps: &InitializeBumps
    ) -> Result<()> {
        self.config.set_inner(Config {
            seed,
            authority,
            locked: false,
            fee,
            config_bump: bumps.config,
            lp_bump: bumps.mint_lp,
            mint_x: self.mint_x.key(),
            mint_y: self.mint_y.key(),
        });
        Ok(())
    }
}
