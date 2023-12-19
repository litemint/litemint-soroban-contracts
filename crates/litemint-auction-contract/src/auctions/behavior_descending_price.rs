/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use crate::types::{AuctionData, DataKey};
use soroban_kit::storage;
use soroban_sdk::Env;

pub struct DescendingPriceAuction;

// DescendingPriceAuction (aka Dutch Auction).
impl super::behavior::BaseAuction for DescendingPriceAuction {
    fn resolve(&self, env: &Env, auction_id: u64) -> bool {
        let auction_data =
            storage::get::<DataKey, AuctionData>(env, &DataKey::AuctionData(auction_id)).unwrap();

        // Auction has expired.
        if auction_data.start_time + auction_data.settings.duration < env.ledger().timestamp() {
            // Finalize with no winner.
            self.finalize(env, auction_id, None)
        } else {
            if let Some(bid) = auction_data.bids.iter().max_by_key(|bid| bid.amount) {
                // Discounted price is met, complete the auction with the winning bid.
                if bid.amount >= self.calculate_price(env, auction_id) {
                    return self.finalize(env, auction_id, Some(&bid));
                }
            }
            false
        }
    }

    fn calculate_price(&self, env: &Env, auction_id: u64) -> i128 {
        let auction_data =
            storage::get::<DataKey, AuctionData>(env, &DataKey::AuctionData(auction_id)).unwrap();
        assert!(
            auction_data.settings.discount_percent > 0
                && auction_data.settings.discount_frequency > 0
        );

        let elapsed = env.ledger().timestamp() - auction_data.start_time;
        let periods = elapsed / auction_data.settings.discount_frequency;
        if auction_data.settings.compounded_discount {
            // Apply compound discount.
            let mut price = auction_data.settings.ask_price;
            for _ in 0..periods {
                price = (100 - auction_data.settings.discount_percent as i128) * price / 100;
            }
            price
        } else {
            // Apply simple linear discount.
            auction_data.settings.ask_price
                * (100 - auction_data.settings.discount_percent * periods as u32) as i128
                / 100
        }
        .max(auction_data.settings.reserve_price) // Ensure price does not fall below reserve.
    }
}
