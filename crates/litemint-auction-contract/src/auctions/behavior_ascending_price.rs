/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use crate::types::{AuctionData, DataKey};
use soroban_sdk::{Address, Env};

use soroban_tools::storage;

pub struct AscendingPriceAuction;

// AscendingPriceAuction (aka English Auction).
impl super::behavior::BaseAuction for AscendingPriceAuction {
    fn resolve(&self, env: &Env, seller: &Address) -> bool {
        let auction_data =
            storage::get::<DataKey, AuctionData>(env, &DataKey::AuctionData(seller.clone()))
                .unwrap();

        // Retrieve the highest bid.
        if let Some(bid) = auction_data.bids.iter().max_by_key(|bid| bid.amount) {
            // Check that the reserve is met and
            // either the auction time has expired or the ask price is met.
            let price = self.calculate_price(&env, &seller);
            if bid.amount >= price
                && (auction_data.start_time + auction_data.duration < env.ledger().timestamp()
                    || (auction_data.ask_price > price && bid.amount >= auction_data.ask_price))
            {
                return self.finalize(env, seller, Some(&bid));
            }
        } else {
            // Auction has expired.
            if auction_data.start_time + auction_data.duration < env.ledger().timestamp() {
                return self.finalize(env, seller, None);
            }
        }
        false
    }

    fn calculate_price(&self, env: &Env, seller: &Address) -> i128 {
        storage::get::<DataKey, AuctionData>(env, &DataKey::AuctionData(seller.clone()))
            .unwrap()
            .reserve_price
    }
}
