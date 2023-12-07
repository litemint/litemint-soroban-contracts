/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use soroban_sdk::{symbol_short, token, Address, Env, Symbol};

use super::behavior_ascending_price::*;
use super::behavior_descending_price::*;
use crate::types::{AdminData, AuctionData, BidData, DataKey};

use soroban_tools::storage;

// Event topics.
const AUCTION: Symbol = symbol_short!("AUCTION");
const BID: Symbol = symbol_short!("BID");

pub mod ledger_times {
    // Assuming 6 seconds average time per ledger.
    pub const LEDGERS_PER_MINUTE: u64 = 10;
    pub const LEDGERS_PER_HOUR: u64 = LEDGERS_PER_MINUTE * 60;
    pub const LEDGERS_PER_DAY: u64 = LEDGERS_PER_HOUR * 24;
    pub const LEDGERS_PER_YEAR: u64 = LEDGERS_PER_DAY * 365;
}

pub trait BaseAuction {
    fn start(&self, env: &Env, seller: &Address, auction_data: &AuctionData) {
        if storage::has::<DataKey, AuctionData>(&env, &DataKey::AuctionData(seller.clone())) {
            panic!("Auction already running.");
        }

        if auction_data.amount == 0 || auction_data.duration == 0 {
            panic!("Invalid auction parameters.");
        }

        // Transfer token to contract.
        let token = token::Client::new(&env, &auction_data.token);
        token.transfer(
            &seller,
            &env.current_contract_address(),
            &auction_data.amount,
        );
        storage::set::<DataKey, AuctionData>(
            env,
            &DataKey::AuctionData(seller.clone()),
            auction_data,
        );

        fn convert_seconds_to_ledgers(watermark: u64) -> u64 {
            watermark
                .checked_add(ledger_times::LEDGERS_PER_MINUTE - 1)
                .and_then(|sum| sum.checked_div(ledger_times::LEDGERS_PER_MINUTE))
                .expect("Invalid duration.")
                .min(ledger_times::LEDGERS_PER_YEAR)
        }

        // Bump the storage according to auction duration,
        // adding a couple hours to avoid expiration with async resolve.
        let expiration_buffer: u64 = 7200;
        storage::bump::<DataKey, AuctionData>(
            env,
            &DataKey::AuctionData(seller.clone()),
            convert_seconds_to_ledgers(auction_data.duration + expiration_buffer),
            convert_seconds_to_ledgers(auction_data.duration + expiration_buffer),
        );

        env.events()
            .publish((AUCTION, symbol_short!("started")), seller);
    }

    fn manage_bid(&self, env: &Env, seller: &Address, buyer: &Address, amount: i128) {
        // First check that the auction is resolved.
        let resolved = self.resolve(env, seller);
        if resolved {
            return;
        }

        let mut auction_data =
            storage::get::<DataKey, AuctionData>(env, &DataKey::AuctionData(seller.clone()))
                .unwrap();
        let market = token::Client::new(&env, &auction_data.market);

        if amount == 0 {
            // Cancel existing bid if amount is zero.
            if let Some(index) = auction_data
                .bids
                .iter()
                .position(|b| b.amount > 0 && b.buyer == *buyer && !b.sniper)
            {
                let bid = &auction_data.bids.get_unchecked(index as u32);
                market.transfer(&env.current_contract_address(), &buyer, &bid.amount);
                auction_data.bids.remove(index as u32);
                env.events()
                    .publish((BID, symbol_short!("deleted")), seller);
            } else {
                panic!("No bid to cancel.");
            }
        } else if amount > 0 && amount >= auction_data.reserve_price {
            if !auction_data
                .bids
                .iter()
                .any(|b| (b.buyer == *buyer && b.amount > 0) || (b.buyer == *buyer && b.sniper))
            {
                market.transfer(&buyer, &env.current_contract_address(), &amount);

                let anti_snipe_time = storage::get::<DataKey, AdminData>(&env, &DataKey::AdminData)
                    .unwrap()
                    .anti_snipe_time;
                let sniper = env.ledger().timestamp()
                    >= auction_data.start_time + auction_data.duration - anti_snipe_time;
                if sniper {
                    auction_data.duration += anti_snipe_time;
                }

                auction_data.bids.push_back(BidData {
                    buyer: buyer.clone(),
                    amount,
                    sniper,
                });
                env.events().publish((BID, symbol_short!("added")), seller);
            } else {
                panic!("Not allowed to place new bid.");
            }
        } else {
            panic!("Invalid bid amount.");
        }

        storage::set::<DataKey, AuctionData>(
            env,
            &DataKey::AuctionData(seller.clone()),
            &auction_data,
        );
        self.resolve(env, seller);
    }

    fn finalize(&self, env: &Env, seller: &Address, winner: Option<&BidData>) -> bool {
        let auction_data =
            storage::get::<DataKey, AuctionData>(env, &DataKey::AuctionData(seller.clone()))
                .unwrap();
        match winner {
            Some(bid) => {
                // We have a winner, transfer token to parties.
                let admin_data =
                    storage::get::<DataKey, AdminData>(&env, &DataKey::AdminData).unwrap();
                let token = token::Client::new(&env, &auction_data.token);
                let market = token::Client::new(&env, &auction_data.market);
                let admin: Address = admin_data.admin;
                let commission_rate: i128 = admin_data.commission_rate;
                let admin_share = bid.amount * commission_rate / 100;
                let seller_share = bid.amount - admin_share;

                token.transfer(
                    &env.current_contract_address(),
                    &bid.buyer,
                    &auction_data.amount,
                );
                market.transfer(&env.current_contract_address(), &admin, &admin_share);
                market.transfer(&env.current_contract_address(), &seller, &seller_share);

                // Cancel all other bids.
                let market = token::Client::new(&env, &auction_data.market);
                for b in auction_data.bids.iter() {
                    if b.amount > 0 && b.buyer != bid.buyer {
                        market.transfer(&env.current_contract_address(), &b.buyer, &b.amount);
                    }
                }

                // Delete the auction.
                storage::remove::<DataKey, AuctionData>(env, &DataKey::AuctionData(seller.clone()));
                env.events()
                    .publish((AUCTION, symbol_short!("won")), seller);
                true
            }
            None => {
                // No winner.
                // Transfer token back to seller.
                let token = token::Client::new(&env, &auction_data.token);
                token.transfer(
                    &env.current_contract_address(),
                    &seller,
                    &auction_data.amount,
                );

                // Cancel all bids.
                let market = token::Client::new(&env, &auction_data.market);
                for bid in auction_data.bids.iter() {
                    if bid.amount > 0 {
                        market.transfer(&env.current_contract_address(), &bid.buyer, &bid.amount);
                    }
                }

                // Delete the auction.
                storage::remove::<DataKey, AuctionData>(env, &DataKey::AuctionData(seller.clone()));
                env.events()
                    .publish((AUCTION, symbol_short!("ended")), seller);
                true
            }
        }
    }

    fn resolve(&self, env: &Env, seller: &Address) -> bool;

    fn calculate_price(&self, _env: &Env, seller: &Address) -> i128;
}

// Using enum/match since no_std prevents the use of dynamic dispatch.
pub enum Dispatcher {
    AscendingPriceAuction,
    DescendingPriceAuction,
}

impl BaseAuction for Dispatcher {
    fn resolve(&self, env: &Env, seller: &Address) -> bool {
        match self {
            Dispatcher::AscendingPriceAuction => AscendingPriceAuction.resolve(env, seller),
            Dispatcher::DescendingPriceAuction => DescendingPriceAuction.resolve(env, seller),
        }
    }

    fn calculate_price(&self, env: &Env, seller: &Address) -> i128 {
        match self {
            Dispatcher::AscendingPriceAuction => AscendingPriceAuction.calculate_price(env, seller),
            Dispatcher::DescendingPriceAuction => {
                DescendingPriceAuction.calculate_price(env, seller)
            }
        }
    }
}

#[macro_export]
macro_rules! dispatcher {
    ($condition:expr) => {
        if $condition {
            Dispatcher::DescendingPriceAuction
        } else {
            Dispatcher::AscendingPriceAuction
        }
    };
}
