/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use crate::types::{AuctionData, DataKey};
use soroban_kit::storage;
use soroban_sdk::Env;

pub struct AscendingPriceAuction;

// AscendingPriceAuction (aka English Auction).
impl super::behavior::BaseAuction for AscendingPriceAuction {
    fn resolve(&self, env: &Env, auction_id: u64) -> bool {
        let auction_data =
            storage::get::<DataKey, AuctionData>(env, &DataKey::AuctionData(auction_id)).unwrap();

        // Retrieve the highest bid.
        if let Some(bid) = auction_data.bids.iter().max_by_key(|bid| bid.amount) {
            // Check that the reserve is met and
            // either the auction time has expired or the ask price is met.
            let price = self.calculate_price(&env, auction_id);
            if bid.amount >= price
                && (auction_data.start_time + auction_data.settings.duration
                    < env.ledger().timestamp()
                    || (auction_data.settings.ask_price > price
                        && bid.amount >= auction_data.settings.ask_price))
            {
                return self.finalize(env, auction_id, Some(&bid));
            }
        } else {
            // Auction has expired.
            if auction_data.start_time + auction_data.settings.duration < env.ledger().timestamp() {
                return self.finalize(env, auction_id, None);
            }
        }
        false
    }

    fn calculate_price(&self, env: &Env, auction_id: u64) -> i128 {
        storage::get::<DataKey, AuctionData>(env, &DataKey::AuctionData(auction_id))
            .unwrap()
            .settings
            .reserve_price
    }
}
