use std::collections::HashMap;

use solana_client::client_error::ClientError;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use spl_associated_token_account::get_associated_token_address;

use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::{
    find_config_program_address,
    state::{RegistryConfig, SetRegistryConfigParams, TOTAL_DISTRIBUTIONS},
};
use everlend_utils::integrations::MoneyMarket;

use crate::{
    accounts_config::{MoneyMarketAccounts, TokenAccounts},
    depositor, general_pool, income_pools, liquidity_oracle, registry, ulp,
    utils::{
        get_asset_maps, spl_create_associated_token_account, spl_token_transfer, Config,
        REFRESH_INCOME_INTERVAL,
    },
};

pub async fn command_create_registry(
    config: &Config,
    keypair: Option<Keypair>,
) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    println!("Fee payer: {}", payer_pubkey);

    let default_accounts = config.get_default_accounts();
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let registry_pubkey = registry::init(config, keypair)?;
    let mut registry_config = SetRegistryConfigParams {
        general_pool_program_id: everlend_general_pool::id(),
        ulp_program_id: everlend_ulp::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
        refresh_income_interval: REFRESH_INCOME_INTERVAL,
    };

    registry_config.money_market_program_ids[0] = default_accounts.port_finance_program_id;
    registry_config.money_market_program_ids[1] = default_accounts.larix_program_id;
    registry_config.money_market_program_ids[2] = default_accounts.solend_program_id;

    println!("registry_config = {:#?}", registry_config);

    registry::set_registry_config(config, &registry_pubkey, registry_config)?;

    initialiazed_accounts.payer = payer_pubkey;
    initialiazed_accounts.registry = registry_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_set_registry_config(
    config: &Config,
    registry_pubkey: Pubkey,
) -> anyhow::Result<()> {
    let default_accounts = config.get_default_accounts();

    let (registry_config_pubkey, _) =
        find_config_program_address(&everlend_registry::id(), &registry_pubkey);
    let registry_config = config.get_account_unpack::<RegistryConfig>(&registry_config_pubkey);
    println!("registry_config = {:#?}", registry_config);

    let mut registry_config_params = SetRegistryConfigParams {
        general_pool_program_id: everlend_general_pool::id(),
        ulp_program_id: everlend_ulp::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
        refresh_income_interval: REFRESH_INCOME_INTERVAL,
    };

    registry_config_params.money_market_program_ids[0] = default_accounts.port_finance_program_id;
    registry_config_params.money_market_program_ids[1] = default_accounts.larix_program_id;
    registry_config_params.money_market_program_ids[2] = default_accounts.solend_program_id;

    println!("registry_config_params = {:#?}", registry_config_params);

    registry::set_registry_config(config, &registry_pubkey, registry_config_params)?;

    Ok(())
}

pub async fn command_create_general_pool_market(
    config: &Config,
    keypair: Option<Keypair>,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let general_pool_market_pubkey = general_pool::create_market(config, keypair)?;

    initialiazed_accounts.general_pool_market = general_pool_market_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_create_income_pool_market(
    config: &Config,
    keypair: Option<Keypair>,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let income_pool_market_pubkey =
        income_pools::create_market(config, keypair, &initialiazed_accounts.general_pool_market)?;

    initialiazed_accounts.income_pool_market = income_pool_market_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_create_mm_pool_market(
    config: &Config,
    keypair: Option<Keypair>,
    money_market: MoneyMarket,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let mm_pool_market_pubkey = ulp::create_market(config, keypair)?;

    initialiazed_accounts.mm_pool_markets[money_market as usize] = mm_pool_market_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_create_liquidity_oracle(
    config: &Config,
    keypair: Option<Keypair>,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let liquidity_oracle_pubkey = liquidity_oracle::init(config, keypair)?;

    initialiazed_accounts.liquidity_oracle = liquidity_oracle_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_create_depositor(
    config: &Config,
    keypair: Option<Keypair>,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let depositor_pubkey = depositor::init(
        config,
        &initialiazed_accounts.registry,
        keypair,
        &initialiazed_accounts.general_pool_market,
        &initialiazed_accounts.income_pool_market,
        &initialiazed_accounts.liquidity_oracle,
    )?;

    initialiazed_accounts.depositor = depositor_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_create_mm_pool(
    config: &Config,
    money_market: MoneyMarket,
    required_mints: Vec<&str>,
) -> anyhow::Result<()> {
    let default_accounts = config.get_default_accounts();
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let (_, collateral_mint_map) = get_asset_maps(default_accounts);
    let money_market_index = money_market as usize;
    let mm_pool_market_pubkey = initialiazed_accounts.mm_pool_markets[money_market_index];

    for key in required_mints {
        let collateral_mint = collateral_mint_map.get(key).unwrap()[money_market_index].unwrap();

        let (mm_pool_pubkey, mm_pool_token_account, mm_pool_mint) =
            ulp::create_pool(config, &mm_pool_market_pubkey, &collateral_mint)?;

        depositor::create_transit(
            config,
            &initialiazed_accounts.depositor,
            &collateral_mint,
            None,
        )?;

        depositor::create_transit(
            config,
            &initialiazed_accounts.depositor,
            &mm_pool_mint,
            None,
        )?;

        let money_market_accounts = MoneyMarketAccounts {
            pool: mm_pool_pubkey,
            pool_token_account: mm_pool_token_account,
            token_mint: collateral_mint,
            pool_mint: mm_pool_mint,
        };

        initialiazed_accounts
            .token_accounts
            .get_mut(key)
            .unwrap()
            .mm_pools[money_market_index] = money_market_accounts;
    }

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_create_token_accounts(
    config: &Config,
    required_mints: Vec<&str>,
) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    let default_accounts = config.get_default_accounts();
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts);

    let mut distribution = DistributionArray::default();
    distribution[0] = 0;
    distribution[1] = 0;
    distribution[2] = 0;

    println!("Prepare borrow authority");
    let (depositor_authority, _) = &everlend_utils::find_program_address(
        &everlend_depositor::id(),
        &initialiazed_accounts.depositor,
    );

    for key in required_mints {
        let mint = mint_map.get(key).unwrap();
        let collateral_mints: Vec<(Pubkey, Pubkey)> = collateral_mint_map
            .get(key)
            .unwrap()
            .iter()
            .zip(initialiazed_accounts.mm_pool_markets.iter())
            .filter_map(|(collateral_mint, mm_pool_market_pubkey)| {
                collateral_mint.map(|coll_mint| (coll_mint, *mm_pool_market_pubkey))
            })
            .collect();

        println!("General pool");
        let (general_pool_pubkey, general_pool_token_account, general_pool_mint) =
            general_pool::create_pool(config, &initialiazed_accounts.general_pool_market, mint)?;

        let token_account = get_associated_token_address(&payer_pubkey, mint);
        println!("Payer token account: {:?}", token_account);
        // let pool_account = get_associated_token_address(&payer_pubkey, &general_pool_mint);
        let pool_account =
            spl_create_associated_token_account(config, &payer_pubkey, &general_pool_mint)
                .unwrap_or_else(|_| {
                    get_associated_token_address(&payer_pubkey, &general_pool_mint)
                });
        println!("Payer pool account: {:?}", pool_account);

        println!("Income pool");
        let (income_pool_pubkey, income_pool_token_account) =
            income_pools::create_pool(config, &initialiazed_accounts.income_pool_market, mint)?;

        // MM Pools
        let mm_pool_pubkeys = collateral_mints
            .iter()
            .map(|(collateral_mint, mm_pool_market_pubkey)| {
                println!("MM Pool: {}", collateral_mint);
                if collateral_mint.eq(&Pubkey::default()) {
                    // We can't skip cuz of mm pools is indexed
                    Ok((Pubkey::default(), Pubkey::default(), Pubkey::default()))
                } else {
                    ulp::create_pool(config, mm_pool_market_pubkey, collateral_mint)
                }
            })
            .collect::<Result<Vec<(Pubkey, Pubkey, Pubkey)>, ClientError>>()?;

        liquidity_oracle::create_token_distribution(
            config,
            &initialiazed_accounts.liquidity_oracle,
            mint,
            &distribution,
        )?;

        // Transit accounts
        let liquidity_transit_pubkey =
            depositor::create_transit(config, &initialiazed_accounts.depositor, mint, None)?;

        // Reserve
        println!("Reserve transit");
        depositor::create_transit(
            config,
            &initialiazed_accounts.depositor,
            mint,
            Some("reserve".to_string()),
        )?;

        println!("Collateral transits");
        collateral_mints
            .iter()
            .filter(|(pk, _)| !pk.eq(&Pubkey::default()))
            .map(|(collateral_mint, _mm_pool_market_pubkey)| {
                depositor::create_transit(
                    config,
                    &initialiazed_accounts.depositor,
                    collateral_mint,
                    None,
                )
            })
            .collect::<Result<Vec<Pubkey>, ClientError>>()?;

        println!("MM Collateral transits");
        mm_pool_pubkeys
            .iter()
            .filter(|(_, _, pk)| !pk.eq(&Pubkey::default()))
            .map(|(_, _, mm_pool_mint)| {
                depositor::create_transit(
                    config,
                    &initialiazed_accounts.depositor,
                    mm_pool_mint,
                    None,
                )
            })
            .collect::<Result<Vec<Pubkey>, ClientError>>()?;

        let mm_pools = collateral_mints
            .iter()
            .zip(mm_pool_pubkeys)
            .map(
                |(
                    (collateral_mint, _mm_pool_market_pubkey),
                    (mm_pool_pubkey, mm_pool_token_account, mm_pool_mint),
                )| {
                    MoneyMarketAccounts {
                        pool: mm_pool_pubkey,
                        pool_token_account: mm_pool_token_account,
                        token_mint: *collateral_mint,
                        pool_mint: mm_pool_mint,
                    }
                },
            )
            .collect();

        // Borrow authorities
        general_pool::create_pool_borrow_authority(
            config,
            &initialiazed_accounts.general_pool_market,
            &general_pool_pubkey,
            depositor_authority,
            10_000, // 100%
        )?;

        initialiazed_accounts.token_accounts.insert(
            key.to_string(),
            TokenAccounts {
                mint: *mint,
                liquidity_token_account: token_account,
                collateral_token_account: pool_account,
                general_pool: general_pool_pubkey,
                general_pool_token_account,
                general_pool_mint,
                income_pool: income_pool_pubkey,
                income_pool_token_account,
                mm_pools,
                liquidity_transit: liquidity_transit_pubkey,
            },
        );
    }

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_add_reserve_liquidity(
    config: &Config,
    mint_key: &str,
    amount: u64,
) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    let default_accounts = config.get_default_accounts();
    let initialiazed_accounts = config.get_initialized_accounts();

    let (mint_map, _) = get_asset_maps(default_accounts);
    let mint = mint_map.get(mint_key).unwrap();

    let (liquidity_reserve_transit_pubkey, _) = everlend_depositor::find_transit_program_address(
        &everlend_depositor::id(),
        &initialiazed_accounts.depositor,
        mint,
        "reserve",
    );

    println!(
        "liquidity_reserve_transit_pubkey = {:?}",
        liquidity_reserve_transit_pubkey
    );

    let token_account = get_associated_token_address(&payer_pubkey, mint);

    println!(
        "Transfer {} lamports of {} to reserve liquidity account",
        amount, mint_key
    );

    spl_token_transfer(
        config,
        &token_account,
        &liquidity_reserve_transit_pubkey,
        amount,
    )?;

    Ok(())
}

pub async fn command_info_reserve_liquidity(config: &Config) -> anyhow::Result<()> {
    let default_accounts = config.get_default_accounts();
    let initialiazed_accounts = config.get_initialized_accounts();

    let (mint_map, _) = get_asset_maps(default_accounts);

    for (_, mint) in mint_map.iter() {
        let (liquidity_reserve_transit_pubkey, _) =
            everlend_depositor::find_transit_program_address(
                &everlend_depositor::id(),
                &initialiazed_accounts.depositor,
                mint,
                "reserve",
            );

        let liquidity_reserve_transit = config
            .get_account_unpack::<spl_token::state::Account>(&liquidity_reserve_transit_pubkey)?;

        println!(
            "{:?}: {:?}",
            liquidity_reserve_transit_pubkey, liquidity_reserve_transit.amount
        );
    }

    Ok(())
}
