/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

#![no_std]

mod auctions; // Auction behaviors and mechanisms.
mod types; // Contract types.

use crate::auctions::{behavior::BaseAuction, behavior::Dispatcher};
use soroban_sdk::{contract, contractimpl, contractmeta, vec, Address, Env, Vec};
use types::{AdminData, AuctionData, BidData, DataKey};

use soroban_tools::storage;

contractmeta!(
    key="desc",
    val="Auction smart contract for the Litemint marketplace, implementing timed auctions with support for both ascending and descending price mechanisms.");

pub trait AuctionContractTrait {
    // Retrieves auction data, if it exists.
    // No authorization required.
    fn get_auction(env: Env, seller: Address) -> Option<AuctionData>;

    // Resolves the auction, applying defined auction behavior and rules.
    // No authorization required.
    fn resolve(env: Env, seller: Address);

    // Places a bid on an auction.
    // Late bids (within anti_snipe_time from the end of the auction)
    // are subject to anti-snipe rules and cannot be cancelled or modified.
    // Buyer authorization required.
    fn place_bid(env: Env, seller: Address, buyer: Address, amount: i128);

    // Extends the duration of an ongoing auction.
    // Seller authorization required.
    fn extend(env: Env, seller: Address, duration: u64) -> bool;

    // One off. Initializes the contract settings post-deployment.
    // Admin authorization required.
    fn initialize(
        env: Env,
        admin: Address,
        anti_snipe_time: u64,
        commission_rate: i128,
        extendable_auctions: bool,
    );

    // Starts a new auction.
    // Behaves as descending price auction if both discount_percent and discount_frequency have non-zero values.
    // The auction can be instantly won if a bidder meets or exceeds the ask_price,
    // provided it is set above the reserve price or discounted below the bid amount (for descending auctions).
    // Seller authorization required.
    fn start(
        env: Env,
        seller: Address,
        token: Address,
        amount: i128,
        duration: u64,
        market: Address,
        reserve_price: i128,
        ask_price: i128,
        discount_percent: u32,
        discount_frequency: u64,
        compounded_discount: bool,
    );

    // Notes: The Litemint marketplace implements an indirection mechanism for
    // auction seller accounts. Learn more: https://blog.litemint.com/anatomy-of-a-stellar-powered-auction-on-litemint/
}

#[contract]
struct AuctionContract;

#[contractimpl]
impl AuctionContractTrait for AuctionContract {
    fn get_auction(env: Env, seller: Address) -> Option<AuctionData> {
        storage::get_or_else::<DataKey, AuctionData, _, _>(
            &env,
            &DataKey::AuctionData(seller),
            |opt| opt,
        )
    }

    fn resolve(env: Env, seller: Address) {
        let auction_data =
            storage::get::<DataKey, AuctionData>(&env, &DataKey::AuctionData(seller.clone()))
                .unwrap();
        dispatcher!(auction_data.discount_percent > 0 && auction_data.discount_frequency > 0)
            .resolve(&env, &seller);
    }

    fn place_bid(env: Env, seller: Address, buyer: Address, amount: i128) {
        buyer.require_auth();

        let auction_data =
            storage::get::<DataKey, AuctionData>(&env, &DataKey::AuctionData(seller.clone()))
                .unwrap();
        dispatcher!(auction_data.discount_percent > 0 && auction_data.discount_frequency > 0)
            .manage_bid(&env, &seller, &buyer, amount);
    }

    fn extend(env: Env, seller: Address, duration: u64) -> bool {
        seller.require_auth();

        if !storage::get_or_else::<DataKey, AdminData, _, _>(&env, &DataKey::AdminData, |opt| {
            opt.unwrap_or_else(|| panic!("Admin not set. Call initialize."))
        })
        .extendable_auctions
        {
            false
        } else {
            let mut auction_data =
                storage::get::<DataKey, AuctionData>(&env, &DataKey::AuctionData(seller.clone()))
                    .unwrap();
            auction_data.duration += duration;
            storage::set::<DataKey, AuctionData>(
                &env,
                &DataKey::AuctionData(seller.clone()),
                &auction_data,
            );
            true
        }
    }

    fn initialize(
        env: Env,
        admin: Address,
        anti_snipe_time: u64,
        commission_rate: i128,
        extendable_auctions: bool,
    ) {
        if storage::has::<DataKey, AdminData>(&env, &DataKey::AdminData) {
            panic!("Admin already set.");
        }

        storage::set::<DataKey, AdminData>(
            &env,
            &DataKey::AdminData,
            &AdminData {
                admin,
                anti_snipe_time: anti_snipe_time.min(60),
                commission_rate: commission_rate.max(0).min(100),
                extendable_auctions,
            },
        );
    }

    fn start(
        env: Env,
        seller: Address,
        token: Address,
        amount: i128,
        duration: u64,
        market: Address,
        reserve_price: i128,
        ask_price: i128,
        discount_percent: u32,
        discount_frequency: u64,
        compounded_discount: bool,
    ) {
        if !storage::has::<DataKey, AdminData>(&env, &DataKey::AdminData) {
            panic!("Admin not set. Call initialize.");
        }

        seller.require_auth();

        let start_time = env.ledger().timestamp();
        let bids: Vec<BidData> = vec![&env];
        dispatcher!(discount_percent > 0 && discount_frequency > 0).start(
            &env,
            &seller,
            &AuctionData {
                token,
                amount,
                duration,
                start_time,
                market,
                reserve_price,
                ask_price,
                discount_percent,
                discount_frequency,
                compounded_discount,
                bids,
            },
        )
    }
}

#[cfg(test)]
mod test;
