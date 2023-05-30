use fuels::{
    accounts::predicate::Predicate,
    prelude::ViewOnlyAccount,
    types::{Address, Bits256},
};

use crate::utils::{
    cotracts_utils::{
        limit_orders_utils::{
            limit_orders_interactions::{cancel_order, create_order},
            LimitOrderPredicateConfigurables,
        },
        token_utils::{token_abi_calls, TokenContract},
    },
    get_balance,
    local_tests_utils::{init_tokens, init_wallets},
    print_title,
};

#[tokio::test]
async fn cancel_order_test() {
    print_title("Recreate Order Test");
    //--------------- WALLETS ---------------
    let wallets = init_wallets().await;
    let admin = &wallets[0];
    let alice = &wallets[1];
    let alice_address = Address::from(alice.address());
    let provider = alice.provider().unwrap();

    println!("alice_address = 0x{:?}\n", alice_address);
    //--------------- TOKENS ---------------
    let assets = init_tokens(&admin).await;
    let usdc = assets.get("USDC").unwrap();
    let usdc_instance = TokenContract::new(usdc.contract_id.into(), admin.clone());
    let uni = assets.get("UNI").unwrap();

    let amount0 = 1000_000_000_u64; //1000 USDC
    let amount1 = 200_000_000_000_u64; //200 UNI
    println!("USDC AssetId (asset0) = 0x{:?}", usdc.asset_id);
    println!("UNI AssetId (asset1) = 0x{:?}", uni.asset_id);
    println!("amount0 = {:?} USDC", amount0 / 1000_000);
    println!("amount1 = {:?} UNI", amount1 / 1000_000_000);

    let price_decimals = 9;
    let exp = (price_decimals + usdc.config.decimals - uni.config.decimals).into();
    let price = amount1 * 10u64.pow(exp) / amount0;
    println!("Price = {:?}\n UNI/USDC", price);

    token_abi_calls::mint_and_transfer(&usdc_instance, amount0, alice_address).await;
    let initial_alice_usdc_balance = get_balance(provider, alice.address(), usdc.asset_id).await;
    println!("Alice minting {:?} USDC\n", amount0 / 1000_000);

    //--------------- PREDICATE ---------

    let configurables = LimitOrderPredicateConfigurables::new()
        .set_ASSET0(Bits256::from_hex_str(&usdc.asset_id.to_string()).unwrap())
        .set_ASSET1(Bits256::from_hex_str(&uni.asset_id.to_string()).unwrap())
        .set_ASSET0_DECINALS(usdc.config.decimals)
        .set_ASSET1_DECINALS(uni.config.decimals)
        .set_MAKER(Bits256::from_hex_str(&alice.address().hash().to_string()).unwrap())
        .set_PRICE(price);

    let predicate: Predicate = Predicate::load_from("./limit-order-predicate/out/debug/limit-order-predicate.bin")
        .unwrap()
        .with_configurables(configurables);
    println!("Predicate root = {:?}\n", predicate.address());
    //--------------- THE TEST ---------
    assert!(alice.get_asset_balance(&usdc.asset_id).await.unwrap() == amount0);
    create_order(&alice, &predicate, &usdc_instance, amount0 / 2)
        .await
        .unwrap();
    println!("Alice transfers 500 USDC to predicate\n");

    create_order(&alice, &predicate, &usdc_instance, amount0 / 2)
        .await
        .unwrap();
    println!("Alice transfers 500 USDC to predicate\n");

    cancel_order(&predicate, &alice, usdc.asset_id, amount0 / 2)
        .await
        .unwrap();

    println!("Alice canceles the order\n");
    // The predicate root's coin has been spent
    let predicate_balance = get_balance(provider, predicate.address(), usdc.asset_id).await;
    assert_eq!(predicate_balance, 0);

    // Wallet balance is the same as before it sent the coins to the predicate
    let wallet_balance = get_balance(provider, alice.address(), usdc.asset_id).await;
    assert_eq!(wallet_balance, initial_alice_usdc_balance);
    println!("Alice balance 1000 UDSC\n");
}
