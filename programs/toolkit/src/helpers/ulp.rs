use crate::utils::get_program_accounts;
use crate::Config;
use everlend_ulp::instruction;
use everlend_ulp::state::{AccountType, Pool, PoolBorrowAuthority};
use solana_client::client_error::ClientError;
use solana_program::instruction::Instruction;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;

const BULK_LIMIT: usize = 30;

pub fn fetch_pools(config: &Config, market_pubkey: &Pubkey) -> Vec<(Pubkey, Pool)> {
    get_program_accounts(
        config,
        &everlend_ulp::id(),
        AccountType::Pool as u8,
        market_pubkey,
    )
    .unwrap()
    .into_iter()
    .filter_map(
        |(address, account)| match Pool::unpack_unchecked(&account.data) {
            Ok(pool) => Some((address, pool)),
            _ => None,
        },
    )
    .collect()
}

pub fn fetch_pool_authorities(
    config: &Config,
    pool_pubkey: &Pubkey,
) -> Vec<(Pubkey, PoolBorrowAuthority)> {
    get_program_accounts(
        config,
        &everlend_ulp::id(),
        AccountType::PoolBorrowAuthority as u8,
        pool_pubkey,
    )
    .unwrap()
    .into_iter()
    .filter_map(
        |(address, account)| match PoolBorrowAuthority::unpack_unchecked(&account.data) {
            Ok(pool_borrow_authority) => Some((address, pool_borrow_authority)),
            _ => None,
        },
    )
    .collect()
}

pub fn bulk_delete_pools(
    config: &Config,
    pool_market: &Pubkey,
    pools: &[(Pubkey, Pool)],
) -> Result<(), ClientError> {
    let instructions: Vec<Instruction> = pools
        .iter()
        .map(|(pool_pubkey, pool)| {
                instruction::delete_pool(
                    &everlend_ulp::id(),
                    pool_market,
                    pool_pubkey,
                    &config.fee_payer.pubkey(),
                    &pool.token_account,
                    &pool.token_mint,
                )
            }
        ).collect();
    let chunks = instructions.chunks(BULK_LIMIT);

    for chunk in chunks {
        let tx = Transaction::new_with_payer(
            chunk,
            Some(&config.fee_payer.pubkey()),
        );

        config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;
    }

    Ok(())
}

pub fn bulk_delete_pool_borrow_authorities(
    config: &Config,
    pool_market: &Pubkey,
    pool: &Pubkey,
    pool_borrow_authorities: &[(Pubkey, PoolBorrowAuthority)],
) -> Result<(), ClientError> {
    let instructions: Vec<Instruction> = pool_borrow_authorities.iter().map(
        |(pool_borrow_authority, _)| {
            instruction::delete_pool_borrow_authority(
                &everlend_ulp::id(),
                pool_market,
                pool,
                pool_borrow_authority,
                &config.fee_payer.pubkey(),
                &config.fee_payer.pubkey(),
            )
        }
    ).collect();
    let chunks = instructions.chunks(BULK_LIMIT);

    for chunk in chunks {
        let tx = Transaction::new_with_payer(
            chunk,
            Some(&config.fee_payer.pubkey()),
        );

        config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;
    }

    Ok(())
}

pub fn delete_pool_market(config: &Config, pool_market: &Pubkey) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[instruction::delete_pool_market(
            &everlend_ulp::id(),
            pool_market,
            &config.fee_payer.pubkey(),
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config
        .sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}