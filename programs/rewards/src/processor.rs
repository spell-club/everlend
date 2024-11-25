//! Program processor

use crate::instruction::RewardsInstruction;
use crate::instructions::*;
use borsh::BorshDeserialize;
use everlend_utils::EverlendError;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::pubkey::Pubkey;

///
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = RewardsInstruction::try_from_slice(input)?;

    match instruction {
        RewardsInstruction::InitializePool { lock_time_sec } => {
            msg!("RewardsInstruction: InitializePool");
            InitializePoolContext::new(program_id, accounts)?.process(program_id, lock_time_sec)
        }
        RewardsInstruction::AddVault {
            ratio_base,
            ratio_quote,
            reward_period_sec,
            distribution_starts_at,
            reward_max_amount_per_period,
        } => {
            msg!("RewardsInstruction: AddVault");
            AddVaultContext::new(program_id, accounts)?.process(
                program_id,
                ratio_base,
                ratio_quote,
                reward_period_sec,
                distribution_starts_at,
                reward_max_amount_per_period,
            )
        }
        RewardsInstruction::FillVault { amount } => {
            msg!("RewardsInstruction: FillVault");
            FillVaultContext::new(program_id, accounts)?.process(program_id, amount)
        }
        RewardsInstruction::DepositMining { amount } => {
            msg!("RewardsInstruction: DepositMining");
            DepositMiningContext::new(program_id, accounts)?.process(program_id, amount)
        }
        RewardsInstruction::WithdrawMining => {
            msg!("RewardsInstruction: WithdrawMining");
            WithdrawMiningContext::new(program_id, accounts)?.process(program_id)
        }
        RewardsInstruction::Claim => {
            msg!("RewardsInstruction: Claim");
            ClaimContext::new(program_id, accounts)?.process(program_id)
        }
        RewardsInstruction::InitializeRoot => {
            msg!("RewardsInstruction: InitializeRoot");
            InitializeRootContext::new(program_id, accounts)?.process(program_id)
        }
        RewardsInstruction::MigratePool => {
            msg!("RewardsInstruction: MigratePool");
            Err(EverlendError::NotImplemented.into())
            // MigratePoolContext::new(program_id, accounts)?.process(program_id)
        }
    }
}
