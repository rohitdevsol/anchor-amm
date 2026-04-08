use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{ Mint, MintTo, Token, TokenAccount, TransferChecked, mint_to, transfer_checked },
};
use constant_product_curve::ConstantProduct;

use crate::{ errors::AmmError, state::Config };

#[derive(Accounts)]
pub struct Deposit<'info> {
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

impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, amount: u64, max_x: u64, max_y: u64) -> Result<()> {
        require!(self.config.locked == false, AmmError::PoolLocked);
        require!(amount != 0, AmmError::InvalidAmount);

        let (x, y) = match
            self.mint_lp.supply == 0 &&
            self.vault_y.amount == 0 &&
            self.vault_x.amount == 0
        {
            true => (max_x, max_y),

            false => {
                let amounts = ConstantProduct::xy_deposit_amounts_from_l(
                    self.vault_x.amount,
                    self.vault_y.amount,
                    self.mint_lp.supply,
                    amount,
                    6
                ).unwrap();

                (amounts.x, amounts.y)
            }
        };

        require!(x <= max_x && y <= max_y, AmmError::SlippageExceeded);

        self.deposit_tokens(true, x)?;

        self.deposit_tokens(false, y)?;

        // Mint LP tokens to user as proof of liquidity provision
        self.mint_lp_tokens(amount)?;

        Ok(())
    }

    fn deposit_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to, mint, decimals) = match is_x {
            true =>
                (
                    self.user_ata_x.to_account_info(),
                    self.vault_x.to_account_info(),
                    self.mint_x.to_account_info(),
                    self.mint_x.decimals,
                ),
            false =>
                (
                    self.user_ata_y.to_account_info(),
                    self.vault_y.to_account_info(),
                    self.mint_y.to_account_info(),
                    self.mint_y.decimals,
                ),
        };

        transfer_checked(
            CpiContext::new(self.token_program.key(), TransferChecked {
                from,
                to,
                mint,
                authority: self.user.to_account_info(),
            }),
            amount,
            decimals
        )
    }

    pub fn mint_lp_tokens(&mut self, amount: u64) -> Result<()> {
        let seeds: &[&[u8]; 3] = &[
            &b"config"[..],
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ];
        let signer_seeds: &[&[&[u8]]] = &[&seeds[..]];

        mint_to(
            CpiContext::new_with_signer(
                self.token_program.key(),
                MintTo {
                    mint: self.mint_lp.to_account_info(),
                    authority: self.config.to_account_info(),
                    to: self.user_ata_lp.to_account_info(),
                },
                signer_seeds
            ),
            amount
        )
    }
}
