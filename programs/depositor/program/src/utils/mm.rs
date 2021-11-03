use crate::error::DepositorError;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, sysvar};
use std::convert::TryInto;

/// Check money market instruction to verify deposit
pub fn check_deposit(instruction: &AccountInfo, amount: u64) -> Result<(), ProgramError> {
    let index = sysvar::instructions::load_current_index_checked(instruction).unwrap();

    // Instruction should be first in transaction
    if index != 0 {
        return Err(DepositorError::InvalidInstructionOrder.into());
    }

    // Load next instruction
    let mm_instruction =
        sysvar::instructions::load_instruction_at_checked((index - 1) as usize, instruction)
            .unwrap();

    // Check that instruction is money market instruction
    // TODO: unfix money market program ids
    if mm_instruction.program_id != spl_token_lending::id() {
        return Err(DepositorError::IncorrectInstructionProgramId.into());
    }

    // TODO: add more checks
    check_amount_from_deposit_instruction(mm_instruction.data, amount)?;

    Ok(())
}

/// Check amount from deposit
pub fn check_amount_from_deposit_instruction(
    instruction_data: Vec<u8>,
    expected_amount: u64,
) -> Result<(), ProgramError> {
    let amount: u64 = u64::from_be_bytes(instruction_data.to_vec().as_slice().try_into().unwrap());
    if amount == expected_amount {
        Ok(())
    } else {
        Err(DepositorError::WrongInstructionAmount.into())
    }
}
