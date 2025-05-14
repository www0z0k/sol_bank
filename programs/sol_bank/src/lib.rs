#![allow(unexpected_cfgs)]  // workaround for [program]
use anchor_lang::prelude::*;

declare_id!("72p9csHh7VeF2yCgsYjcwNzujaQpPvLKJZ9c5v6nz9CV");

#[program]
pub mod sol_bank {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let acct = &mut ctx.accounts.user_account;
        acct.authority = ctx.accounts.user.key();
        acct.balance = 0;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        // 1) Gather infos
        let user_key = ctx.accounts.user.key();
        let ua_key   = ctx.accounts.user_account.key();
        let user_info= ctx.accounts.user.to_account_info();
        let ua_info  = ctx.accounts.user_account.to_account_info();

        // 2) CPI: transfer lamports from user → PDA
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &user_key,
            &ua_key,
            amount,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[user_info, ua_info],
        )?;

        // 3) Update on-chain balance
        let acct = &mut ctx.accounts.user_account;
        acct.balance = acct
            .balance
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        // 1) Check & update internal balance
        let acct = &mut ctx.accounts.user_account;
        require!(acct.balance >= amount, ErrorCode::InsufficientFunds);
        acct.balance = acct.balance.checked_sub(amount).unwrap();

        // 2) Borrow lamports mutably
        let ua_info   = ctx.accounts.user_account.to_account_info();
        let user_info = ctx.accounts.user.to_account_info();

        // Debit the PDA’s lamports:
        {
            let mut lamports_ref = ua_info.try_borrow_mut_lamports()?;
            // lamports_ref: RefMut<&mut u64>, so `*lamports_ref` is &mut u64
            let lamports_mut: &mut u64 = &mut *lamports_ref;
            *lamports_mut = lamports_mut
                .checked_sub(amount)
                .ok_or(ErrorCode::InsufficientFunds)?;
        }

        // Credit the user’s account:
        {
            let mut lamports_ref = user_info.try_borrow_mut_lamports()?;
            let lamports_mut: &mut u64 = &mut *lamports_ref;
            *lamports_mut = lamports_mut
                .checked_add(amount)
                .ok_or(ErrorCode::Overflow)?;
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = user,
        space = 8 + 32 + 8,
        seeds = [b"user-account", user.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        mut,
        seeds = [b"user-account", user.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [b"user-account", user.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct UserAccount {
    pub authority: Pubkey,
    pub balance:    u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Insufficient funds for withdrawal")]
    InsufficientFunds,
    #[msg("Balance overflow")]
    Overflow,
}
