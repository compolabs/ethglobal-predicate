use fuels::{
    accounts::predicate::Predicate,
    prelude::ViewOnlyAccount,
    types::{Address, Bits256},
};
use spark_sdk::{
    limit_orders_utils::{
        limit_orders_interactions::create_order, LimitOrderPredicateConfigurables,
    },
    proxy_utils::{deploy_proxy_contract, ProxySendFundsToPredicateParams},
};
use src20_sdk::{TokenContract, token_abi_calls};

use crate::utils::{
    local_tests_utils::{init_tokens, init_wallets},
    print_title,
};

#[tokio::test]
async fn create_order_test() {
    print_title("Create Order Test");
    //--------------- WALLETS ---------------
    let wallets = init_wallets().await;
    let admin = &wallets[0];
    let alice = &wallets[1];
    let alice_address = Address::from(alice.address());

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
    println!("Price = {:?} UNI/USDC", price);

    token_abi_calls::mint(&usdc_instance, amount0, alice_address)
        .await
        .unwrap();
    println!("Alice minting {:?} USDC\n", amount0 / 1000_000);

    //--------------- PREDICATE ---------

    let configurables = LimitOrderPredicateConfigurables::new()
        .set_ASSET0(Bits256::from_hex_str(&usdc.asset_id.to_string()).unwrap())
        .set_ASSET1(Bits256::from_hex_str(&uni.asset_id.to_string()).unwrap())
        .set_ASSET0_DECIMALS(usdc.config.decimals)
        .set_ASSET1_DECIMALS(uni.config.decimals)
        .set_MAKER(Bits256::from_hex_str(&alice.address().hash().to_string()).unwrap())
        .set_PRICE(price);

    let predicate: Predicate =
        Predicate::load_from("./limit-order-predicate/out/debug/limit-order-predicate.bin")
            .unwrap()
            .with_configurables(configurables)
            .with_provider(admin.provider().unwrap().to_owned());
    println!("Predicate root = {:?}\n", predicate.address());
    //--------------- THE TEST ---------
    assert!(alice.get_asset_balance(&usdc.asset_id).await.unwrap() == amount0);

    let params = ProxySendFundsToPredicateParams {
        predicate_root: predicate.address().into(),
        asset_0: usdc.contract_id.into(),
        asset_1: uni.contract_id.into(),
        maker: alice_address,
        min_fulfill_amount_0: 1,
        price,
        asset_0_decimals: 6,
        asset_1_decimals: 9,
        price_decimals: 9,
    };

    let proxy = deploy_proxy_contract(alice, "proxy-contract/out/debug/proxy-contract.bin").await;
    let proxy_address = format!("0x{}", proxy.contract_id().hash);
    println!("proxyAddress = {:?}", proxy_address);
    create_order(&alice, &proxy_address, params, amount0)
        .await
        .unwrap();

    assert!(alice.get_asset_balance(&usdc.asset_id).await.unwrap() == 0);
    assert!(predicate.get_asset_balance(&usdc.asset_id).await.unwrap() == amount0);
}
