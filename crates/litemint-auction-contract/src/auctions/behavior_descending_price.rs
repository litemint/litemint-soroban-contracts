/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use crate::types::{AuctionData, DataKey};
use soroban_sdk::{Address, Env};

use soroban_tools::storage;

pub struct DescendingPriceAuction;

// DescendingPriceAuction (aka Dutch Auction).
impl super::behavior::BaseAuction for DescendingPriceAuction {
    fn resolve(&self, env: &Env, seller: &Address) -> bool {
        let auction_data =
            storage::get::<DataKey, AuctionData>(env, &DataKey::AuctionData(seller.clone()))
                .unwrap();

        // Auction has expired.
        if auction_data.start_time + auction_data.duration < env.ledger().timestamp() {
            // Finalize with no winner.
            self.finalize(env, seller, None)
        } else {
            if let Some(bid) = auction_data.bids.iter().max_by_key(|bid| bid.amount) {
                // Discounted price is met, complete the auction with the winning bid.
                if bid.amount >= self.calculate_price(env, seller) {
                    return self.finalize(env, seller, Some(&bid));
                }
            }
            false
        }
    }

    fn calculate_price(&self, env: &Env, seller: &Address) -> i128 {
        let auction_data =
            storage::get::<DataKey, AuctionData>(env, &DataKey::AuctionData(seller.clone()))
                .unwrap();

        // Sanity checks.
        if auction_data.discount_percent == 0 || auction_data.discount_frequency == 0 {
            panic!("Invalid parameters.");
        } else {
            let elapsed = env.ledger().timestamp() - auction_data.start_time;
            let periods = elapsed / auction_data.discount_frequency;
            if auction_data.compounded_discount {
                // Apply compound discount.
                let mut price = auction_data.ask_price;
                for _ in 0..periods {
                    price = (100 - auction_data.discount_percent as i128) * price / 100;
                }
                price
            } else {
                // Apply simple linear discount.
                auction_data.ask_price
                    * (100 - auction_data.discount_percent * periods as u32) as i128
                    / 100
            }
        }
        .max(auction_data.reserve_price) // Ensure price does not fall below reserve.
    }
}
