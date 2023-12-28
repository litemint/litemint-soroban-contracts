/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use crate::{
    types::{Compensation, LicenseStatus, MarketData, Terms},
    RoyaltyContract, RoyaltyContractClient,
};
extern crate std;

use core::panic::AssertUnwindSafe;
use soroban_sdk::{
    testutils::{Address as _, Logs},
    token, Address, Env,
};
use std::{panic::catch_unwind, println};
use token::{Client as TokenClient, StellarAssetClient as TokenAdminClient};

fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let contract_address = e.register_stellar_asset_contract(admin.clone());
    (
        TokenClient::new(e, &contract_address),
        TokenAdminClient::new(e, &contract_address),
    )
}

fn create_royalty_contract(e: &Env) -> RoyaltyContractClient {
    RoyaltyContractClient::new(e, &e.register_contract(None, RoyaltyContract {}))
}

#[test]
fn test_compensation_fixed_royalties() {
    let env = Env::default();
    env.mock_all_auths();

    let royalty_interest = 188; // Licensor: 183 Fixed
    let commission_rate = 3; // Admin: 3%
    let transfer_fee = 100;
    let admin = Address::generate(&env);
    let licensor = Address::generate(&env);
    let licensee = Address::generate(&env);
    let nft_issuer = Address::generate(&env);

    let (property, property_client) = create_token_contract(&env, &nft_issuer);
    let (lien, lien_client) = create_token_contract(&env, &nft_issuer);
    let (market, market_client) = create_token_contract(&env, &admin);

    property_client.mint(&licensor, &1);
    lien_client.mint(&licensor, &1);
    market_client.mint(&licensee, &(royalty_interest + transfer_fee));

    // Create the terms for royalties with a fixed compensation model.
    let terms = Terms {
        licensor: licensor.clone(),
        property: property.address.clone(),
        lien: lien.address.clone(),
        compensation: Compensation::Fixed,
        royalty_interest,
        currency: market.address.clone(),
        transfer_fee,
        recur_period: 0,
        grace_period: 60,
    };

    let royalty_contract = create_royalty_contract(&env);
    royalty_contract.initialize(&admin, &commission_rate);
    royalty_contract.add_property(&terms);

    // Trying to initialize again should panic.
    let result = catch_unwind(AssertUnwindSafe(|| {
        royalty_contract.initialize(&admin, &commission_rate);
    }));
    assert!(result.is_err(), "Already initialized.");

    // Trying to add same property again should panic.
    let result = catch_unwind(AssertUnwindSafe(|| {
        royalty_contract.add_property(&terms);
    }));
    assert!(result.is_err(), "Already added.");

    // Execute the royalty agreement.
    let mut license = royalty_contract.execute(&terms.property);

    // License terms should match.
    assert_eq!(license.terms, terms);
    assert_eq!(license.status, LicenseStatus::Paid);
    assert_eq!(license.licensee, licensor);
    assert_eq!(license.created_time, env.ledger().timestamp());
    assert_eq!(license.recur_time, 0);
    assert_eq!(
        license.grace_time,
        license.created_time + terms.grace_period
    );

    // Transfer NFT to licensee and execute agreement.
    // Status should now be Unpaid.
    property.transfer(&licensor, &licensee, &1);
    license = royalty_contract.execute(&terms.property);
    assert_eq!(license.status, LicenseStatus::Unpaid);

    // Make the royalty payment.
    // License status should now be Paid and new licensee should be `licensee`.
    license = royalty_contract.pay(&terms.property, &licensee);
    assert_eq!(license.status, LicenseStatus::Paid);
    assert_eq!(license.licensee, licensee);

    // Check balances.
    let admin_share_fixed = royalty_interest
        .checked_mul(commission_rate)
        .and_then(|val| val.checked_add(99))
        .and_then(|val| val.checked_div(100))
        .unwrap()
        .max(1);

    let admin_share_transfer = transfer_fee
        .checked_mul(commission_rate)
        .and_then(|val| val.checked_add(99))
        .and_then(|val| val.checked_div(100))
        .unwrap()
        .max(1);

    assert_eq!(market.balance(&licensee), 0);
    assert_eq!(
        market.balance(&licensor),
        (royalty_interest - admin_share_fixed) + (transfer_fee - admin_share_transfer)
    );
    assert_eq!(
        market.balance(&royalty_contract.address),
        admin_share_fixed + admin_share_transfer
    );
    assert_eq!(lien.balance(&royalty_contract.address), 1i128);

    // Print all.
    println!("{}", env.logs().all().join("\n"));
    println!("{:?}", env.budget());
}

#[test]
fn test_compensation_subscription_royalties() {
    let env = Env::default();
    env.mock_all_auths();

    let royalty_interest = 100;
    let commission_rate = 3;
    let transfer_fee = 100;
    let admin = Address::generate(&env);
    let licensor = Address::generate(&env);
    let licensee = Address::generate(&env);
    let nft_issuer = Address::generate(&env);

    let (property, property_client) = create_token_contract(&env, &nft_issuer);
    let (lien, lien_client) = create_token_contract(&env, &nft_issuer);
    let (market, market_client) = create_token_contract(&env, &admin);

    property_client.mint(&licensor, &1);
    lien_client.mint(&licensor, &1);
    market_client.mint(&licensee, &(royalty_interest * 2 + transfer_fee));

    // Create the terms for royalties with a subscription compensation model.
    let terms = Terms {
        licensor: licensor.clone(),
        property: property.address.clone(),
        lien: lien.address.clone(),
        compensation: Compensation::Subscription,
        royalty_interest,
        transfer_fee,
        currency: market.address.clone(),
        recur_period: 706,
        grace_period: 60,
    };

    let royalty_contract = create_royalty_contract(&env);
    royalty_contract.initialize(&admin, &commission_rate);
    royalty_contract.add_property(&terms);

    let mut license = royalty_contract.execute(&terms.property);
    assert_eq!(license.status, LicenseStatus::Paid);

    // Licensee becomes token owner.
    property.transfer(&licensor, &licensee, &1);
    license = royalty_contract.pay(&terms.property, &licensee);
    assert_eq!(license.status, LicenseStatus::Paid);
    assert_eq!(license.licensee, licensee);

    // Calling execute on expired recur_time sets the license
    // status to unpaid.
    license = royalty_contract.execute(&terms.property);
    assert_eq!(license.status, LicenseStatus::Unpaid);

    // Make the recurring payment.
    license = royalty_contract.pay(&terms.property, &licensee);
    assert_eq!(license.status, LicenseStatus::Paid);

    let admin_share_fixed = royalty_interest
        .checked_mul(commission_rate)
        .and_then(|val| val.checked_add(99))
        .and_then(|val| val.checked_div(100))
        .unwrap()
        .max(1);

    let admin_share_transfer = transfer_fee
        .checked_mul(commission_rate)
        .and_then(|val| val.checked_add(99))
        .and_then(|val| val.checked_div(100))
        .unwrap()
        .max(1);

    // Licensor balance should show 2 payments.
    assert_eq!(
        market.balance(&licensor),
        royalty_interest * 2 - admin_share_transfer * 2 + (transfer_fee - admin_share_transfer)
    );
    assert_eq!(
        market.balance(&royalty_contract.address),
        admin_share_transfer + admin_share_fixed * 2
    );

    // Lien still with contract.
    assert_eq!(lien.balance(&royalty_contract.address), 1);
    assert_eq!(market.balance(&licensee), 0);
}

#[test]
fn test_compensation_percentage_royalties() {
    let env = Env::default();
    env.mock_all_auths();

    let royalty_interest = 10;
    let commission_rate = 3;
    let transfer_fee = 100;
    let admin = Address::generate(&env);
    let licensor = Address::generate(&env);
    let licensee = Address::generate(&env);
    let nft_issuer = Address::generate(&env);

    let (property, property_client) = create_token_contract(&env, &nft_issuer);
    let (lien, lien_client) = create_token_contract(&env, &nft_issuer);
    let (market, market_client) = create_token_contract(&env, &admin);

    let market_data = MarketData {
        price: 88888,
        asset: market.address.clone(),
    };

    property_client.mint(&licensor, &1);
    lien_client.mint(&licensor, &1);
    let royalties_to_pay = market_data
        .price
        .checked_mul(royalty_interest)
        .and_then(|val| val.checked_add(99))
        .and_then(|val| val.checked_div(100))
        .unwrap();
    market_client.mint(&licensee, &(royalties_to_pay + transfer_fee));

    // Create the terms for royalties with a percentage compensation model.
    let terms = Terms {
        licensor: licensor.clone(),
        property: property.address.clone(),
        lien: lien.address.clone(),
        compensation: Compensation::Percentage,
        royalty_interest,
        transfer_fee,
        currency: market.address.clone(),
        recur_period: 0,
        grace_period: 60,
    };

    let royalty_contract = create_royalty_contract(&env);
    royalty_contract.initialize(&admin, &commission_rate);
    royalty_contract.add_property(&terms);

    let mut license = royalty_contract.execute(&terms.property);
    assert_eq!(license.status, LicenseStatus::Paid);

    // Simulate an oracle price feed.
    royalty_contract.test_oracle_feed(&property.address, &market_data.price, &market_data.asset);

    // Licensee becomes token owner.
    property.transfer(&licensor, &licensee, &1);
    license = royalty_contract.pay(&terms.property, &licensee);
    assert_eq!(license.status, LicenseStatus::Paid);
    assert_eq!(license.licensee, licensee);

    let admin_share_percent = royalties_to_pay
        .checked_mul(commission_rate)
        .and_then(|val| val.checked_add(99))
        .and_then(|val| val.checked_div(100))
        .unwrap()
        .max(1);

    let admin_share_transfer = transfer_fee
        .checked_mul(commission_rate)
        .and_then(|val| val.checked_add(99))
        .and_then(|val| val.checked_div(100))
        .unwrap()
        .max(1);

    assert_eq!(
        market.balance(&licensor),
        royalties_to_pay - admin_share_percent + transfer_fee - admin_share_transfer
    );
    assert_eq!(
        market.balance(&royalty_contract.address),
        admin_share_percent + admin_share_transfer
    );

    // Lien still with contract.
    assert_eq!(lien.balance(&royalty_contract.address), 1);
    assert_eq!(market.balance(&licensee), 0);
}

#[test]
fn test_payment_enforcement() {
    let env = Env::default();
    env.mock_all_auths();

    let royalty_interest = 100;
    let commission_rate = 3;
    let admin = Address::generate(&env);
    let licensor = Address::generate(&env);
    let licensee = Address::generate(&env);
    let nft_issuer = Address::generate(&env);

    let (property, property_client) = create_token_contract(&env, &nft_issuer);
    let (lien, lien_client) = create_token_contract(&env, &nft_issuer);
    let (market, market_client) = create_token_contract(&env, &admin);

    property_client.mint(&licensor, &1);
    lien_client.mint(&licensor, &1);
    market_client.mint(&licensee, &royalty_interest);

    // Create the terms for royalties with a fixed compensation model.
    let terms = Terms {
        licensor: licensor.clone(),
        property: property.address.clone(),
        lien: lien.address.clone(),
        compensation: Compensation::Fixed,
        royalty_interest,
        transfer_fee: 0,
        currency: market.address.clone(),
        recur_period: 0,
        grace_period: 1473, // LATE
    };

    let royalty_contract = create_royalty_contract(&env);
    royalty_contract.initialize(&admin, &commission_rate);
    royalty_contract.add_property(&terms);
    royalty_contract.execute(&terms.property);

    // Transfer NFT to licensee and execute agreement.
    // Status should be Unpaid as royalty payment was not made.
    property.transfer(&licensor, &licensee, &1);
    let mut license = royalty_contract.execute(&terms.property);
    assert_eq!(license.status, LicenseStatus::Unpaid);

    // Grace period expires on that call (test value 1473).
    license = royalty_contract.pay(&terms.property, &licensee);
    assert_eq!(license.status, LicenseStatus::Breached);

    // Balances should remain untouched.
    assert_eq!(market.balance(&licensor), 0);
    assert_eq!(market.balance(&licensee), royalty_interest);

    // Licensor should have receive the lien
    // that allows seizing the propert via on-chain pre-auth.
    assert_eq!(license.licensee, licensor);
    assert_eq!(lien.balance(&licensor), 1i128);
}
