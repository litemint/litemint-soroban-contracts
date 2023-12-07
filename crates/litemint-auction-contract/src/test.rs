/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use crate::{types::AuctionData, AuctionContract, AuctionContractClient};
extern crate std;

use core::panic::AssertUnwindSafe;
use soroban_sdk::{
    testutils::{Address as _, Logs},
    token, vec, Address, Env,
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
    auction_data: &AuctionData,
    seller: &Address,
) {
    auction_contract.start(
        &seller,
        &auction_data.token,
        &auction_data.amount,
        &auction_data.duration,
        &auction_data.market,
        &auction_data.reserve_price,
        &auction_data.ask_price,
        &auction_data.discount_percent,
        &auction_data.discount_frequency,
        &auction_data.compounded_discount,
    );
}

#[test]
fn test_auctions() {
    let env = Env::default();
    env.mock_all_auths();

    let initial_balance = 1000;
    let commission_rate = 10;
    let token_supply: i128 = 5;
    let extendable_auctions = true;
    let token_admin = Address::random(&env);
    let seller = Address::random(&env);
    let (token, token_admin_client) = create_token_contract(&env, &token_admin);
    let (market, market_admin_client) = create_token_contract(&env, &token_admin);
    let auction_contract = create_auction_contract(&env);
    let bidders = [Address::random(&env), Address::random(&env)];

    // Initialize the balances.
    token_admin_client.mint(&seller, &token_supply);
    for bidder in bidders.iter() {
        market_admin_client.mint(&bidder, &initial_balance);
    }

    // Initialize the contract. Sets the admin, anti_snipe_time (in seconds)
    // and commission_rate (in percent).
    auction_contract.initialize(&token_admin, &300, &commission_rate, &extendable_auctions);

    // No auction found should return None.
    assert_eq!(auction_contract.get_auction(&seller), None);

    // Configure a descending price auction (Dutch auction).
    let mut auction_data: AuctionData = AuctionData {
        token: token.address.clone(),
        amount: 1,
        duration: 180,
        start_time: env.ledger().timestamp(),
        market: market.address.clone(),
        reserve_price: 100,
        ask_price: 900,
        discount_percent: 10,
        discount_frequency: 20,
        compounded_discount: false,
        bids: vec![&env],
    };

    // Start the auction.
    start_auction(&env, &auction_contract, &auction_data, &seller);

    // Should be matching all auction parameters.
    let mut test_auction = auction_contract.get_auction(&seller);
    match test_auction {
        Some(test_auction) => {
            assert_eq!(test_auction, auction_data);
        }
        None => {}
    }

    // Placing a zero bid should panic if no existing bid to cancel.
    let mut result = catch_unwind(AssertUnwindSafe(|| {
        auction_contract.place_bid(&seller, &bidders[0], &0);
    }));
    assert!(result.is_err(), "No bid to cancel.");

    // Placing a bid below reserve should always panic.
    result = catch_unwind(AssertUnwindSafe(|| {
        auction_contract.place_bid(&seller, &bidders[0], &(auction_data.reserve_price - 1));
    }));
    assert!(result.is_err(), "Invalid bid amount.");

    // Placing a bid at or above reserve.
    auction_contract.place_bid(&seller, &bidders[0], &(auction_data.reserve_price));

    // Canceling the bid.
    auction_contract.place_bid(&seller, &bidders[0], &0);

    // Placing a new bid (bidder 1).
    auction_contract.place_bid(&seller, &bidders[0], &(auction_data.reserve_price + 1));

    // Placing a new bid (bidder 2).
    auction_contract.place_bid(&seller, &bidders[1], &(auction_data.reserve_price + 2));

    test_auction = auction_contract.get_auction(&seller);
    match test_auction {
        Some(test_auction) => {
            // There should be 2 live bids at that point.
            assert_eq!(test_auction.bids.len(), 2);

            // Check the balances.
            assert_eq!(
                market.balance(&bidders[0]),
                initial_balance - auction_data.reserve_price - 1
            );
            assert_eq!(
                market.balance(&bidders[1]),
                initial_balance - auction_data.reserve_price - 2
            );
            assert_eq!(
                market.balance(&auction_contract.address),
                (auction_data.reserve_price + 1) * 2 + 1
            );
            assert_eq!(
                token.balance(&auction_contract.address),
                auction_data.amount
            );
        }
        None => {}
    }

    // Try to resolve the auction.
    // The auction should still be running as no bidder
    // matched the ask_price.
    auction_contract.resolve(&seller);

    // Verify that no transfer occured.
    test_auction = auction_contract.get_auction(&seller);
    match test_auction {
        Some(test_auction) => {
            assert_eq!(test_auction.bids.len(), 2);

            // Verify that balances remain unchanged.
            assert_eq!(
                market.balance(&bidders[0]),
                initial_balance - auction_data.reserve_price - 1
            );
            assert_eq!(
                market.balance(&bidders[1]),
                initial_balance - auction_data.reserve_price - 2
            );
            assert_eq!(
                market.balance(&auction_contract.address),
                (auction_data.reserve_price + 1) * 2 + 1
            );
            assert_eq!(
                token.balance(&auction_contract.address),
                auction_data.amount
            );
        }
        None => {}
    }

    // Cancel the bids.
    auction_contract.place_bid(&seller, &bidders[0], &0);
    auction_contract.place_bid(&seller, &bidders[1], &0);

    // Submit a concurrent invalid bid.
    auction_contract.place_bid(&seller, &bidders[0], &(auction_data.ask_price - 1));

    // Submit a winning bid.
    auction_contract.place_bid(&seller, &bidders[1], &(auction_data.ask_price));

    // Auction should have been resolved immediately.
    test_auction = auction_contract.get_auction(&seller);
    match test_auction {
        Some(_test_auction) => {
            assert!(false, "Auction should not be running.");
        }
        None => {}
    }

    // Try to resolve the auction (should have no effect).
    result = catch_unwind(AssertUnwindSafe(|| {
        auction_contract.resolve(&seller);
    }));
    assert!(result.is_err(), "No auction to resolve.");

    // Verify all balances to check the auction executed properly.
    assert_eq!(
        market.balance(&bidders[1]),
        initial_balance - auction_data.ask_price
    );
    assert_eq!(token.balance(&bidders[1]), auction_data.amount);
    assert_eq!(market.balance(&bidders[0]), initial_balance);
    assert_eq!(market.balance(&auction_contract.address), 0);
    assert_eq!(token.balance(&auction_contract.address), 0);
    assert_eq!(
        market.balance(&token_admin),
        auction_data.ask_price * commission_rate / 100
    );
    assert_eq!(
        market.balance(&seller),
        auction_data.ask_price * (100 - commission_rate) / 100
    );
    assert_eq!(token.balance(&seller), token_supply - 1);

    // Start an ascending price auction.
    auction_data.discount_percent = 0;
    auction_data.discount_frequency = 0;
    start_auction(&env, &auction_contract, &auction_data, &seller);

    // Place a bid at ask price (buy now).
    auction_contract.place_bid(&seller, &bidders[0], &(auction_data.ask_price));

    // The auction should have resolved as ask price is met.
    test_auction = auction_contract.get_auction(&seller);
    match test_auction {
        Some(_test_auction) => {
            assert!(false, "Auction should not be running.");
        }
        None => {}
    }

    // Try to resolve the auction (should have no effect).
    result = catch_unwind(AssertUnwindSafe(|| {
        auction_contract.resolve(&seller);
    }));
    assert!(result.is_err(), "No auction to resolve.");

    // Verify the balances.
    assert_eq!(
        market.balance(&bidders[0]),
        initial_balance - auction_data.ask_price
    );
    assert_eq!(token.balance(&bidders[0]), auction_data.amount);
    assert_eq!(market.balance(&auction_contract.address), 0);
    assert_eq!(token.balance(&auction_contract.address), 0);
    assert_eq!(
        market.balance(&token_admin),
        (auction_data.ask_price * commission_rate / 100) * 2
    );
    assert_eq!(
        market.balance(&seller),
        (auction_data.ask_price * (100 - commission_rate) / 100) * 2
    );
    assert_eq!(token.balance(&seller), token_supply - 2);

    // Start a new ascending price auction.
    start_auction(&env, &auction_contract, &auction_data, &seller);

    // Place a bid at reserve price.
    auction_contract.place_bid(&seller, &bidders[0], &(auction_data.reserve_price));

    // Auction should continue to run as the duration has not elapsed.
    auction_contract.resolve(&seller);

    // Extend the auction duration.
    auction_contract.extend(&seller, &auction_data.duration);
    test_auction = auction_contract.get_auction(&seller);
    match test_auction {
        Some(_test_auction) => {
            assert_eq!(_test_auction.duration, auction_data.duration * 2);
        }
        None => {}
    }

    // Print all.
    println!("{:?}", env.budget());
    println!("{}", env.logs().all().join("\n"));
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
    let token_admin = Address::random(&env);
    let seller = Address::random(&env);
    let (token, token_admin_client) = create_token_contract(&env, &token_admin);
    let (market, market_admin_client) = create_token_contract(&env, &token_admin);
    let auction_contract = create_auction_contract(&env);
    let bidders = [Address::random(&env), Address::random(&env)];

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

    // No auction found should return None.
    assert_eq!(auction_contract.get_auction(&seller), None);

    // Configure a descending price auction (Dutch auction).
    let auction_data: AuctionData = AuctionData {
        token: token.address.clone(),
        amount: 1,
        duration: duration,
        start_time: env.ledger().timestamp(),
        market: market.address.clone(),
        reserve_price: 100,
        ask_price: 900,
        discount_percent: 10,
        discount_frequency: 20,
        compounded_discount: false,
        bids: vec![&env],
    };

    println!("START {}", auction_data.start_time);

    // Start the auction.
    start_auction(&env, &auction_contract, &auction_data, &seller);

    // Submit a winning bid.
    auction_contract.place_bid(&seller, &bidders[0], &(auction_data.reserve_price));
    let test_auction = auction_contract.get_auction(&seller);
    match test_auction {
        Some(test_auction) => {
            // There should be 1 live bid at that point.
            assert_eq!(test_auction.bids.len(), 1);

            // The sniper flag should be set on the bid.
            assert_eq!(test_auction.bids.first().unwrap().sniper, true);

            // Should not be able to cancel a sniper bid.
            let result = catch_unwind(AssertUnwindSafe(|| {
                auction_contract.place_bid(&seller, &bidders[0], &0);
            }));
            assert!(result.is_err(), "Bid could be cancelled, should not.");
        }
        None => {}
    }
}
