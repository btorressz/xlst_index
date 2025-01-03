use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, MintTo, Burn, Transfer};

declare_id!("4Va57RmHYYpqyFxR5jkGWV65mBNcdCtmVmHJj4tmSgxF");

#[program]
pub mod xlst_index {
    use super::*;

    /// Initialize 
     pub fn initialize(ctx: Context<Initialize>, config: ConfigInput) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        global_state.admin = ctx.accounts.admin.key();
        global_state.base_yield_rate = config.base_yield_rate;
        global_state.index_token_mint = ctx.accounts.index_token_mint.key();

        msg!("Protocol Initialized by Admin: {}", global_state.admin);
        emit!(ProtocolInitialized {
            admin: global_state.admin,
            base_yield_rate: global_state.base_yield_rate,
        });
        Ok(())
    }

    /// Mint xLST tokens
 pub fn mint_xlst(ctx: Context<MintXlst>, amount: u64) -> Result<()> {
     //   require!(amount > 0, CustomError::InvalidAmount);
         require!(amount > 0, CustomError::ZeroAmount);  

        {
            // Transfer collateral from the user to the protocol's liquidity pool
            token::transfer(
                ctx.accounts.into_transfer_to_pool_context(),
                amount,
            )?;
        }

        {
            // Mint xLST tokens to the user's account
            token::mint_to(
                ctx.accounts.into_mint_to_context(),
                amount,
            )?;
        }

        // Safely update user balance after transfers are complete
        let user = &mut ctx.accounts.user_account;
        user.balance += amount;
        msg!("Minted {} xLST tokens to user: {}", amount, user.owner);
        Ok(())
    }


    /// Burn xLST tokens
   pub fn burn_xlst(ctx: Context<BurnXlst>, amount: u64) -> Result<()> {
        {
            let user = &mut ctx.accounts.user_account;
            require!(user.balance >= amount, CustomError::InsufficientBalance);
        }

        {
            // Burn xLST tokens from user's account
            token::burn(ctx.accounts.into_burn_context(), amount)?;
        }

        // Safely update user balance after burn
        let user = &mut ctx.accounts.user_account;
        user.balance -= amount;
        msg!("Burned {} xLST tokens from user: {}", amount, user.owner);
        Ok(())
    }
    /// Update Yield Rate
    pub fn update_yield(ctx: Context<UpdateYield>, new_yield_rate: u64) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        require!(ctx.accounts.admin.key() == global_state.admin, CustomError::Unauthorized);

        global_state.base_yield_rate = new_yield_rate;
        msg!("Updated yield rate to: {}", new_yield_rate);
        Ok(())
    }

    /// Swap Tokens in AMM Pool
   pub fn swap(ctx: Context<Swap>, amount_in: u64, min_amount_out: u64) -> Result<()> {
     //   require!(amount_in > 0, CustomError::InvalidAmount);
     //   require!(min_amount_out > 0, CustomError::InvalidAmount);
         require!(amount_in > 0, CustomError::ZeroAmount);  // Changed from InvalidAmount to ZeroAmount
         require!(min_amount_out > 0, CustomError::ZeroAmount);
        
        let pool = &mut ctx.accounts.liquidity_pool;

        let amount_out = (pool.sol_balance * amount_in) / (pool.xlst_balance + amount_in);
        require!(amount_out >= min_amount_out, CustomError::InsufficientOutputAmount);

        pool.xlst_balance += amount_in;
        pool.sol_balance -= amount_out;

        msg!("Swapped {} xLST for {} SOL", amount_in, amount_out);
        Ok(())
    }
}


/// Accounts for Initialize
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = admin, space = 8 + 48)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(init, payer = admin, mint::decimals = 9, mint::authority = admin)]
    pub index_token_mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

/// Accounts for Mint xLST
#[derive(Accounts)]
pub struct MintXlst<'info> {
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut)]
    pub liquidity_pool: Account<'info, LiquidityPool>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub protocol_token_account: Account<'info, TokenAccount>,
    pub index_token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

/// Accounts for Burn xLST
#[derive(Accounts)]
pub struct BurnXlst<'info> {
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub index_token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

/// Accounts for Update Yield
#[derive(Accounts)]
pub struct UpdateYield<'info> {
    #[account(mut)]
    pub global_state: Account<'info, GlobalState>,
    #[account(signer)]
    pub admin: Signer<'info>,
}

/// Accounts for Swap
#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub liquidity_pool: Account<'info, LiquidityPool>,
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
    pub token_program: Program<'info, Token>,
}

/// Global Protocol State
#[account]
pub struct GlobalState {
    pub admin: Pubkey,
    pub base_yield_rate: u64,
    pub index_token_mint: Pubkey,
}

/// User Account
#[account]
pub struct UserAccount {
    pub owner: Pubkey,
    pub balance: u64,
}

/// Liquidity Pool Account
#[account]
pub struct LiquidityPool {
    pub xlst_balance: u64,
    pub sol_balance: u64,
    pub stablecoin_balance: u64,
}

/// Config Input for Initialize
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ConfigInput {
    pub base_yield_rate: u64,
}

/// Custom Errors
#[error_code]
pub enum CustomError {
    #[msg("Insufficient balance to perform the operation.")]
    InsufficientBalance,
    #[msg("Unauthorized operation.")]
    Unauthorized,
    #[msg("Insufficient output amount for swap.")]
    InsufficientOutputAmount,
    #[msg("Amount must be greater than zero.")]
    ZeroAmount,
}

#[event]
pub struct ProtocolInitialized {
    pub admin: Pubkey,
    pub base_yield_rate: u64,
}

/// Implement Helper Contexts for Token Operations
impl<'info> MintXlst<'info> {
    fn into_transfer_to_pool_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_token_account.to_account_info(),
                to: self.protocol_token_account.to_account_info(),
                authority: self.user_account.to_account_info(),
            },
        )
    }

    fn into_mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            MintTo {
                mint: self.index_token_mint.to_account_info(),
                to: self.user_token_account.to_account_info(),
                authority: self.global_state.to_account_info(),
            },
        )
    }
}

impl<'info> BurnXlst<'info> {
    fn into_burn_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Burn {
                mint: self.index_token_mint.to_account_info(),
                from: self.user_token_account.to_account_info(),
                authority: self.user_account.to_account_info(),
            },
        )
    }
}