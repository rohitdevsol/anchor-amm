use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{ Burn, Mint, Token, TokenAccount, TransferChecked, burn, transfer_checked },
};
use constant_product_curve::ConstantProduct;

use crate::{ errors::AmmError, state::Config };

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_x: Account<'info, Mint>,

    #[account(mint::token_program = token_program)]
    pub mint_y: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_x, 
        associated_token::authority = config, 
        associated_token::token_program = token_program 
    )]
    pub vault_x: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
        associated_token::token_program = token_program
    )]
    pub vault_y: Account<'info, TokenAccount>,

    #[account(mut, seeds = [b"lp", config.key().as_ref()], bump = config.lp_bump)]
    pub mint_lp: Account<'info, Mint>,

    #[account(
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
        has_one = mint_x,
        has_one = mint_y
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut , 
        associated_token::mint = mint_x,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_ata_x: Box<Account<'info, TokenAccount>>,

    #[account(
        mut , 
        associated_token::mint = mint_y,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_ata_y: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
        associated_token::token_program = token_program
    )]
    pub user_ata_lp: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64, min_x: u64, min_y: u64) -> Result<()> {
        require!(self.config.locked == false, AmmError::PoolLocked);
        require!(amount != 0, AmmError::InvalidAmount);

        // Calculate token amounts to withdraw based on current pool state
        let (x, y) = match
            self.mint_lp.supply == 0 &&
            self.vault_x.amount == 0 &&
            self.vault_y.amount == 0
        {
            true => (min_x, min_y),

            false => {
                let amounts = ConstantProduct::xy_withdraw_amounts_from_l(
                    self.vault_x.amount,
                    self.vault_y.amount,
                    self.mint_lp.supply,
                    amount,
                    6
                ).unwrap();
                (amounts.x, amounts.y)
            }
        };

        require!(x >= min_x && y >= min_y, AmmError::SlippageExceeded);

        // Burn LP tokens from user's account first
        self.burn_lp_tokens(amount)?;

        // Transfer calculated amounts of both tokens to user
        self.withdraw_tokens(x, true)?; // Transfer token X
        self.withdraw_tokens(y, false)?; // Transfer token Y
        Ok(())
    }

    pub fn burn_lp_tokens(&mut self, amount: u64) -> Result<()> {
        // Create signer seeds for config PDA
        let signer_seeds: &[&[&[u8]]] = &[
            &[b"config", &self.config.seed.to_le_bytes(), &[self.config.config_bump]],
        ];

        burn(
            CpiContext::new_with_signer(
                self.token_program.key(),
                Burn {
                    mint: self.mint_lp.to_account_info(),
                    from: self.user_ata_lp.to_account_info(),
                    authority: self.user.to_account_info(),
                },
                signer_seeds
            ),
            amount
        )
    }
    pub fn withdraw_tokens(&mut self, amount: u64, is_x: bool) -> Result<()> {
        let (from, to, mint, decimals) = match is_x {
            true =>
                (
                    self.vault_x.to_account_info(), // Transfer from vault X
                    self.user_ata_x.to_account_info(), // Transfer to user's X account
                    self.mint_x.to_account_info(), // Token X mint
                    self.mint_x.decimals, // Token X decimals
                ),
            false =>
                (
                    self.vault_y.to_account_info(), // Transfer from vault Y
                    self.user_ata_y.to_account_info(), // Transfer to user's Y account
                    self.mint_y.to_account_info(), // Token Y mint
                    self.mint_y.decimals, // Token Y decimals
                ),
        };

        // Create signer seeds for config PDA
        let signer_seeds: &[&[&[u8]]] = &[
            &[b"config", &self.config.seed.to_le_bytes(), &[self.config.config_bump]],
        ];

        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.key(),
                TransferChecked {
                    from,
                    to,
                    mint,
                    authority: self.config.to_account_info(), // Config PDA signs the transfer
                },
                signer_seeds
            ),
            amount,
            decimals
        )
    }
}
