/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use crate::{types::{AuctionData, AuctionSettings}, AuctionContract, AuctionContractClient};
extern crate std;

use core::panic::AssertUnwindSafe;
use soroban_sdk::{
    testutils::{Address as _, Logs},
    token, vec, Address, Env, Bytes, BytesN,
};
use std::panic::catch_unwind;
use std::println;
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let contract_address = e.register_stellar_asset_contract(admin.clone());
    (
        TokenClient::new(e, &contract_address),
        TokenAdminClient::new(e, &contract_address),
    )
}
fn create_auction_contract(e: &Env) -> AuctionContractClient {
    AuctionContractClient::new(e, &e.register_contract(None, AuctionContract {}))
}

fn start_auction(
    _env: &Env,
    auction_contract: &AuctionContractClient,
    auction_data: &AuctionSettings
) -> u64 {
    auction_contract.start(auction_data)
}

#[test]
fn test_ascending_descending_auctions() {
    let env = Env::default();
    env.mock_all_auths();

    let initial_balance = 1000;
    let commission_rate = 10;
    let token_supply: i128 = 5;
    let extendable_auctions = true;
    let token_admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let (token, token_admin_client) = create_token_contract(&env, &token_admin);
    let (market, market_admin_client) = create_token_contract(&env, &token_admin);
    let auction_contract = create_auction_contract(&env);
    let bidders = [Address::generate(&env), Address::generate(&env)];

    // Initialize the balances.
    token_admin_client.mint(&seller, &token_supply);
    for bidder in bidders.iter() {
        market_admin_client.mint(&bidder, &initial_balance);
    }

    // Initialize the contract. Sets the admin, anti_snipe_time (in seconds)
    // and commission_rate (in percent).
    auction_contract.initialize(&token_admin, &300, &commission_rate, &extendable_auctions);

    // Configure a descending price auction (Dutch auction).
    let mut auction_settings: AuctionSettings = AuctionSettings {
        seller: seller.clone(),
        token: token.address.clone(),
        amount: 1,
        duration: 180,
        market: market.address.clone(),
        reserve_price: 100,
        ask_price: 900,
        discount_percent: 10,
        discount_frequency: 20,
        compounded_discount: false,
        sealed_phase_time: 0,
        sealed_bid_deposit: 0,
    };

    // Start the auction.
    let mut auction_id = start_auction(&env, &auction_contract, &auction_settings);

    // Should be matching all auction parameters.
    let mut test_auction = auction_contract.get_auction(&auction_id);
    match test_auction {
        Some(test_auction) => {
            assert_eq!(test_auction, AuctionData::new(auction_settings.clone(), 0, vec![&env], vec![&env], auction_id));
        }
        None => {}
    }

    // Placing a zero bid should panic if no existing bid to cancel.
    let mut result = catch_unwind(AssertUnwindSafe(|| {
        auction_contract.place_bid(&auction_id, &bidders[0], &0, &None);
    }));
    assert!(result.is_err(), "No bid to cancel.");

    // Placing a bid below reserve should always panic.
    result = catch_unwind(AssertUnwindSafe(|| {
        auction_contract.place_bid(&auction_id, &bidders[0], &(auction_settings.reserve_price - 1), &None);
    }));
    assert!(result.is_err(), "Invalid bid amount.");

    // Placing a bid at or above reserve.
    auction_contract.place_bid(&auction_id, &bidders[0], &(auction_settings.reserve_price), &None);

    // Canceling the bid.
    auction_contract.place_bid(&auction_id, &bidders[0], &0, &None);

    // Placing a new bid (bidder 1).
    auction_contract.place_bid(&auction_id, &bidders[0], &(auction_settings.reserve_price + 1), &None);

    // Placing a new bid (bidder 2).
    auction_contract.place_bid(&auction_id, &bidders[1], &(auction_settings.reserve_price + 2), &None);

    test_auction = auction_contract.get_auction(&auction_id);
    match test_auction {
        Some(test_auction) => {
            // There should be 2 live bids at that point.
            assert_eq!(test_auction.bids.len(), 2);

            // Check the balances.
            assert_eq!(
                market.balance(&bidders[0]),
                initial_balance - auction_settings.reserve_price - 1
            );
            assert_eq!(
                market.balance(&bidders[1]),
                initial_balance - auction_settings.reserve_price - 2
            );
            assert_eq!(
                market.balance(&auction_contract.address),
                (auction_settings.reserve_price + 1) * 2 + 1
            );
            assert_eq!(
                token.balance(&auction_contract.address),
                auction_settings.amount
            );
        }
        None => {}
    }

    // Try to resolve the auction.
    // The auction should still be running as no bidder
    // matched the ask_price.
    auction_contract.resolve(&auction_id);

    // Verify that no transfer occured.
    test_auction = auction_contract.get_auction(&auction_id);
    match test_auction {
        Some(test_auction) => {
            assert_eq!(test_auction.bids.len(), 2);

            // Verify that balances remain unchanged.
            assert_eq!(
                market.balance(&bidders[0]),
                initial_balance - auction_settings.reserve_price - 1
            );
            assert_eq!(
                market.balance(&bidders[1]),
                initial_balance - auction_settings.reserve_price - 2
            );
            assert_eq!(
                market.balance(&auction_contract.address),
                (auction_settings.reserve_price + 1) * 2 + 1
            );
            assert_eq!(
                token.balance(&auction_contract.address),
                auction_settings.amount
            );
        }
        None => {}
    }

    // Cancel the bids.
    auction_contract.place_bid(&auction_id, &bidders[0], &0, &None);
    auction_contract.place_bid(&auction_id, &bidders[1], &0, &None);

    // Submit a concurrent invalid bid.
    auction_contract.place_bid(&auction_id, &bidders[0], &(auction_settings.ask_price - 1), &None);

    // Submit a winning bid.
    auction_contract.place_bid(&auction_id, &bidders[1], &(auction_settings.ask_price), &None);

    // Auction should have been resolved immediately.
    test_auction = auction_contract.get_auction(&auction_id);
    match test_auction {
        Some(_test_auction) => {
            assert!(false, "Auction should not be running.");
        }
        None => {}
    }

    // Try to resolve the auction (should have no effect).
    result = catch_unwind(AssertUnwindSafe(|| {
        auction_contract.resolve(&auction_id);
    }));
    assert!(result.is_err(), "No auction to resolve.");

    // Verify all balances to check the auction executed properly.
    assert_eq!(
        market.balance(&bidders[1]),
        initial_balance - auction_settings.ask_price
    );
    assert_eq!(token.balance(&bidders[1]), auction_settings.amount);
    assert_eq!(market.balance(&bidders[0]), initial_balance);
    assert_eq!(market.balance(&auction_contract.address), 0);
    assert_eq!(token.balance(&auction_contract.address), 0);
    assert_eq!(
        market.balance(&token_admin),
        auction_settings.ask_price * commission_rate / 100
    );
    assert_eq!(
        market.balance(&seller),
        auction_settings.ask_price * (100 - commission_rate) / 100
    );
    assert_eq!(token.balance(&seller), token_supply - 1);

    // Start an ascending price auction.
    auction_settings.discount_percent = 0;
    auction_settings.discount_frequency = 0;
    auction_id = start_auction(&env, &auction_contract, &auction_settings);

    // Place a bid at ask price (buy now).
    auction_contract.place_bid(&auction_id, &bidders[0], &(auction_settings.ask_price), &None);

    // The auction should have resolved as ask price is met.
    test_auction = auction_contract.get_auction(&auction_id);
    match test_auction {
        Some(_test_auction) => {
            assert!(false, "Auction should not be running.");
        }
        None => {}
    }

    // Try to resolve the auction (should have no effect).
    result = catch_unwind(AssertUnwindSafe(|| {
        auction_contract.resolve(&auction_id);
    }));
    assert!(result.is_err(), "No auction to resolve.");

    // Verify the balances.
    assert_eq!(
        market.balance(&bidders[0]),
        initial_balance - auction_settings.ask_price
    );
    assert_eq!(token.balance(&bidders[0]), auction_settings.amount);
    assert_eq!(market.balance(&auction_contract.address), 0);
    assert_eq!(token.balance(&auction_contract.address), 0);
    assert_eq!(
        market.balance(&token_admin),
        (auction_settings.ask_price * commission_rate / 100) * 2
    );
    assert_eq!(
        market.balance(&seller),
        (auction_settings.ask_price * (100 - commission_rate) / 100) * 2
    );
    assert_eq!(token.balance(&seller), token_supply - 2);

    // Start a new ascending price auction.
    auction_id = start_auction(&env, &auction_contract, &auction_settings);

    // Place a bid at reserve price.
    auction_contract.place_bid(&auction_id, &bidders[0], &(auction_settings.reserve_price), &None);

    // Auction should continue to run as the duration has not elapsed.
    auction_contract.resolve(&auction_id);

    // Extend the auction duration.
    auction_contract.extend(&auction_id, &auction_settings.duration);
    test_auction = auction_contract.get_auction(&auction_id);
    match test_auction {
        Some(_test_auction) => {
            assert_eq!(_test_auction.settings.duration, auction_settings.duration * 2);
        }
        None => {}
    }

    // Print all.    
    println!("{}", env.logs().all().join("\n"));
    println!("{:?}", env.budget());
}

#[test]
fn test_sealed_bid_auctions() {
    let env = Env::default();
    env.mock_all_auths();

    let duration = 50;
    let initial_balance = 1000;
    let commission_rate = 10;
    let extendable_auctions = true;
    let token_supply: i128 = 5;
    let token_admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let (token, token_admin_client) = create_token_contract(&env, &token_admin);
    let (market, market_admin_client) = create_token_contract(&env, &token_admin);
    let auction_contract = create_auction_contract(&env);
    let bidders = [Address::generate(&env), Address::generate(&env)];

    // Initialize the balances.
    token_admin_client.mint(&seller, &token_supply);
    for bidder in bidders.iter() {
        market_admin_client.mint(&bidder, &initial_balance);
    }

    // Initialize the contract. Sets the admin, anti_snipe_time (in seconds)
    // and commission_rate (in percent).
    auction_contract.initialize(
        &token_admin,
        &duration,
        &commission_rate,
        &extendable_auctions,
    );

    // Configure a sealed bid auction.
    let auction_settings: AuctionSettings = AuctionSettings {
        seller,
        token: token.address.clone(),
        amount: 1,
        duration: duration,
        market: market.address.clone(),
        reserve_price: 100,
        ask_price: 900,
        discount_percent: 0,
        discount_frequency: 0,
        compounded_discount: false,
        sealed_phase_time : 1,
        sealed_bid_deposit: 10,
    };

    // Start the auction.
    let auction_id = start_auction(&env, &auction_contract, &auction_settings);

    // Placing a regular bid or trying to reveal a bid should panic
    let result = catch_unwind(AssertUnwindSafe(|| {
        auction_contract.place_bid(&auction_id, &bidders[0], &(auction_settings.reserve_price), &None);
    }));
    assert!(result.is_err(), "Should panic. Invalid phase + no bids to reveal");

    // Bid is sealed with sha256([big_endian_amount;16][salt;32][big_endian_auction_id;8]).
    let mut test_auction = auction_contract.get_auction(&auction_id).unwrap();

    let salt = BytesN::from_array(&env, &[0_u8; 32]);
    let mut sealed_data = Bytes::from_array(&env, &auction_settings.reserve_price.to_be_bytes());
    sealed_data.append(&Bytes::from_slice(&env, &salt.to_array())); // Salt.
    sealed_data.append(&Bytes::from_slice(&env, &test_auction.id.to_be_bytes()));
    let hash = env.crypto().sha256(&sealed_data);

    // Submit a sealed bid.    
    auction_contract.place_sealed_bid(&auction_id, &bidders[0], &hash);

    // Verify the sealed bid deposit was made against the contract.
    test_auction = auction_contract.get_auction(&auction_id).unwrap();
    assert_eq!(test_auction.deposits.len(), 1);
    assert_eq!(test_auction.deposits.first().unwrap().amount, auction_settings.sealed_bid_deposit);
    assert_eq!(test_auction.deposits.first().unwrap().buyer, bidders[0]);

    // Verify the balances.
    assert_eq!(market.balance(&bidders[0]), initial_balance - auction_settings.sealed_bid_deposit);
    assert_eq!(market.balance(&auction_contract.address), auction_settings.sealed_bid_deposit);

    // Reveal the bid
    auction_contract.place_bid(&auction_id, &bidders[0], &(auction_settings.reserve_price), &Some(salt));

    // The revealed bid amount should now be recorded.
    test_auction = auction_contract.get_auction(&auction_id).unwrap();
    assert_eq!(test_auction.deposits.len(), 0);
    assert_eq!(test_auction.bids.len(), 1);
    assert_eq!(test_auction.bids.first().unwrap().amount, auction_settings.reserve_price);
    assert_eq!(test_auction.bids.first().unwrap().buyer, bidders[0]);

    // Verify the balances.
    // The deposit should have been paid back and the full bid amount
    // collected on the contract address.
    assert_eq!(market.balance(&bidders[0]), initial_balance - auction_settings.reserve_price);
    assert_eq!(market.balance(&auction_contract.address), auction_settings.reserve_price);
}

#[test]
fn test_anti_sniping() {
    let env = Env::default();
    env.mock_all_auths();

    let duration = 50;
    let initial_balance = 1000;
    let commission_rate = 10;
    let extendable_auctions = true;
    let token_supply: i128 = 5;
    let token_admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let (token, token_admin_client) = create_token_contract(&env, &token_admin);
    let (market, market_admin_client) = create_token_contract(&env, &token_admin);
    let auction_contract = create_auction_contract(&env);
    let bidders = [Address::generate(&env), Address::generate(&env)];

    // Initialize the balances.
    token_admin_client.mint(&seller, &token_supply);
    for bidder in bidders.iter() {
        market_admin_client.mint(&bidder, &initial_balance);
    }

    // Initialize the contract. Sets the admin, anti_snipe_time (in seconds)
    // and commission_rate (in percent).
    auction_contract.initialize(
        &token_admin,
        &duration,
        &commission_rate,
        &extendable_auctions,
    );

    // Configure a descending price auction (Dutch auction).
    let auction_settings: AuctionSettings = AuctionSettings {
        seller,
        token: token.address.clone(),
        amount: 1,
        duration: duration,
        market: market.address.clone(),
        reserve_price: 100,
        ask_price: 900,
        discount_percent: 10,
        discount_frequency: 20,
        compounded_discount: false,
        sealed_phase_time : 0,
        sealed_bid_deposit: 0,
    };

    // Start the auction.
    let auction_id = start_auction(&env, &auction_contract, &auction_settings);

    // Submit a winning bid.
    auction_contract.place_bid(&auction_id, &bidders[0], &(auction_settings.reserve_price), &None);
    let test_auction = auction_contract.get_auction(&auction_id);
    match test_auction {
        Some(test_auction) => {
            // There should be 1 live bid at that point.
            assert_eq!(test_auction.bids.len(), 1);

            // The sniper flag should be set on the bid.
            assert_eq!(test_auction.bids.first().unwrap().sniper, true);

            // Should not be able to cancel a sniper bid.
            let result = catch_unwind(AssertUnwindSafe(|| {
                auction_contract.place_bid(&auction_id, &bidders[0], &0, &None);
            }));
            assert!(result.is_err(), "Bid could be cancelled, should not.");
        }
        None => {}
    }
}
