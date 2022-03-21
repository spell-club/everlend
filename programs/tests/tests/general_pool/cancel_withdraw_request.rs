#![cfg(feature = "test-bpf")]

use solana_program::instruction::InstructionError;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

use everlend_general_pool::{
    find_transit_program_address, find_user_withdrawal_request_program_address, instruction,
    state::WithdrawalRequest,
};
use everlend_utils::EverlendError;

use crate::utils::*;

const INITIAL_USER_BALANCE: u64 = 5000000;
const WITHDRAWAL_REQUEST_RENT: u64 = 1670400;

async fn setup() -> (
    ProgramTestContext,
    TestGeneralPoolMarket,
    TestGeneralPool,
    TestGeneralPoolBorrowAuthority,
    LiquidityProvider,
) {
    let mut context = presetup().await.0;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let test_pool = TestGeneralPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let test_pool_borrow_authority =
        TestGeneralPoolBorrowAuthority::new(&test_pool, context.payer.pubkey());
    test_pool_borrow_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
            ULP_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    let user = add_liquidity_provider(
        &mut context,
        &test_pool.token_mint_pubkey,
        &test_pool.pool_mint.pubkey(),
        101,
    )
    .await
    .unwrap();

    transfer(&mut context, &user.owner.pubkey(), INITIAL_USER_BALANCE)
        .await
        .unwrap();

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    (
        context,
        test_pool_market,
        test_pool,
        test_pool_borrow_authority,
        user,
    )
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    let user_account = get_account(&mut context, &user.owner.pubkey()).await;
    assert_eq!(user_account.lamports, INITIAL_USER_BALANCE);

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 50, 1)
        .await
        .unwrap();

    let user_account = get_account(&mut context, &user.owner.pubkey()).await;
    assert_eq!(
        user_account.lamports,
        INITIAL_USER_BALANCE - WITHDRAWAL_REQUEST_RENT
    );

    let withdraw_requests = test_pool
        .get_withdraw_requests(
            &mut context,
            &test_pool_market,
            &everlend_general_pool::id(),
        )
        .await;
    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    let withdraw_request = test_pool
        .get_user_withdraw_requests(
            &mut context,
            &test_pool_market,
            1,
            &everlend_general_pool::id(),
        )
        .await;

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        50
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 50,);
    assert_eq!(withdraw_requests.last_request_id, 1);
    assert_eq!(withdraw_requests.last_processed_request_id, 0,);
    assert_eq!(withdraw_requests.liquidity_supply, 50,);
    assert_eq!(
        withdraw_request,
        WithdrawalRequest {
            rent_payer: user.owner.pubkey(),
            source: user.pool_account,
            destination: user.token_account,
            liquidity_amount: 50,
            collateral_amount: 50,
        }
    );

    context.warp_to_slot(3 + 3).unwrap();

    test_pool
        .cancel_withdraw_request(&mut context, &test_pool_market, &user, 1)
        .await
        .unwrap();

    let withdraw_requests = test_pool
        .get_withdraw_requests(
            &mut context,
            &test_pool_market,
            &everlend_general_pool::id(),
        )
        .await;

    let (user_withdraw_request, _) = find_user_withdrawal_request_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.token_mint_pubkey,
        1,
    );

    let wth_account = context
        .banks_client
        .get_account(user_withdraw_request)
        .await
        .unwrap();

    assert_eq!(wth_account, None);
    assert_eq!(withdraw_requests.last_processed_request_id, 1);
    assert_eq!(withdraw_requests.liquidity_supply, 0);
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 0);
    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        100
    );

    let user_account = get_account(&mut context, &user.owner.pubkey()).await;
    assert_eq!(user_account.lamports, INITIAL_USER_BALANCE);
}

#[tokio::test]
async fn fail_with_invalid_pool_market() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    let withdraw_index = 1;

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 50, withdraw_index)
        .await
        .unwrap();

    context.warp_to_slot(3 + 3).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::cancel_withdraw_request(
            &everlend_general_pool::id(),
            &Pubkey::new_unique(),
            &test_pool.pool_pubkey,
            &user.pool_account,
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &test_pool_market.manager.pubkey(),
            &user.owner.pubkey(),
            withdraw_index,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_market.manager],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32)
        )
    )
}

#[tokio::test]
async fn fail_with_invalid_pool() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    let withdraw_index = 1;

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 50, withdraw_index)
        .await
        .unwrap();

    context.warp_to_slot(3 + 3).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::cancel_withdraw_request(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &Pubkey::new_unique(),
            &user.pool_account,
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &test_pool_market.manager.pubkey(),
            &user.owner.pubkey(),
            withdraw_index,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_market.manager],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32)
        )
    )
}

#[tokio::test]
async fn fail_with_invalid_source() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    let withdraw_index = 1;

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 50, withdraw_index)
        .await
        .unwrap();

    context.warp_to_slot(3 + 3).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::cancel_withdraw_request(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &Pubkey::new_unique(),
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &test_pool_market.manager.pubkey(),
            &user.owner.pubkey(),
            withdraw_index,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_market.manager],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    )
}

#[tokio::test]
async fn fail_with_invalid_token_mint() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    let withdraw_index = 1;

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 50, withdraw_index)
        .await
        .unwrap();

    context.warp_to_slot(3 + 3).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::cancel_withdraw_request(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.pool_account,
            &Pubkey::new_unique(),
            &test_pool.pool_mint.pubkey(),
            &test_pool_market.manager.pubkey(),
            &user.owner.pubkey(),
            withdraw_index,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_market.manager],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    )
}

#[tokio::test]
async fn fail_with_invalid_pool_mint() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    let withdraw_index = 1;

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 50, withdraw_index)
        .await
        .unwrap();

    context.warp_to_slot(3 + 3).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::cancel_withdraw_request(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.pool_account,
            &test_pool.token_mint_pubkey,
            &Pubkey::new_unique(),
            &test_pool_market.manager.pubkey(),
            &user.owner.pubkey(),
            withdraw_index,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_market.manager],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidAccountData)
    )
}

#[tokio::test]
async fn fail_with_wrong_manager() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    let withdraw_index = 1;

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 50, withdraw_index)
        .await
        .unwrap();

    context.warp_to_slot(3 + 3).unwrap();

    let wrong_manager = Keypair::new();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::cancel_withdraw_request(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.pool_account,
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &wrong_manager.pubkey(),
            &user.owner.pubkey(),
            withdraw_index,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wrong_manager],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    )
}

#[tokio::test]
async fn fail_with_fake_pool_market() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    let fake_pool_market = TestGeneralPoolMarket::new();
    fake_pool_market.init(&mut context).await.unwrap();

    let withdraw_index = 1;

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 50, withdraw_index)
        .await
        .unwrap();

    context.warp_to_slot(3 + 3).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::cancel_withdraw_request(
            &everlend_general_pool::id(),
            &fake_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.pool_account,
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &fake_pool_market.manager.pubkey(),
            &user.owner.pubkey(),
            withdraw_index,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &fake_pool_market.manager],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    )
}
