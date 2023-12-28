/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use soroban_kit::{
    commit, fsm, fsm::StateMachine, reveal, soroban_tools, state_machine, storage,
    TransitionHandler,
};
use soroban_sdk::{symbol_short, token, Address, Bytes, BytesN, Env, Symbol};

use crate::types::{AdminData, AuctionData, AuctionPhase, AuctionRegion, BidData, DataKey};

use super::behavior_ascending_price::*;
use super::behavior_descending_price::*;

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
    fn start(&self, env: &Env, auction_id: u64, auction_data: &AuctionData) {
        assert!(!storage::has::<DataKey, AuctionData>(
            &env,
            &DataKey::AuctionData(auction_id)
        ));
        assert!(auction_data.settings.amount > 0 && auction_data.settings.duration > 0);

        // Transfer token to contract.
        let token = token::Client::new(&env, &auction_data.settings.token);
        token.transfer(
            &auction_data.settings.seller,
            &env.current_contract_address(),
            &auction_data.settings.amount,
        );
        storage::set::<DataKey, AuctionData>(env, &DataKey::AuctionData(auction_id), auction_data);

        fn convert_seconds_to_ledgers(seconds: u64) -> u64 {
            seconds
                .checked_add(ledger_times::LEDGERS_PER_MINUTE - 1)
                .and_then(|sum| sum.checked_div(ledger_times::LEDGERS_PER_MINUTE))
                .expect("Invalid duration.")
                .min(ledger_times::LEDGERS_PER_YEAR)
        }

        // Extend data TTL according to auction duration,
        // adding a couple hours to avoid expiration with async resolve.
        let expiration_buffer: u64 = 7200;
        storage::extend_ttl::<DataKey, AuctionData>(
            env,
            &DataKey::AuctionData(auction_id),
            convert_seconds_to_ledgers(auction_data.settings.duration + expiration_buffer) as u32,
            convert_seconds_to_ledgers(auction_data.settings.duration + expiration_buffer) as u32,
        );

        env.events()
            .publish((AUCTION, symbol_short!("started")), auction_id);

        // Set the auction phase.
        let region = AuctionRegion::Dispatcher(auction_id);
        let state_machine =
            StateMachine::<AuctionRegion, AuctionPhase>::new(&region, fsm::StorageType::Instance);
        match self.is_sealed_bid_auction(&auction_data) {
            true => {
                state_machine.set_state(&env, &AuctionPhase::Committing);
            }
            false => {
                state_machine.set_state(&env, &AuctionPhase::Running);
            }
        }
    }

    fn place_sealed_bid(
        &self,
        env: &Env,
        auction_id: u64,
        buyer: &Address,
        sealed_amount: &BytesN<32>,
    ) {
        self.commit_bid(env, sealed_amount);

        let mut auction_data =
            storage::get::<DataKey, AuctionData>(env, &DataKey::AuctionData(auction_id)).unwrap();

        if auction_data
            .deposits
            .iter()
            .any(|b| b.buyer == *buyer && b.amount > 0)
        {
            panic!("Not allowed");
        } else {
            // Deposit the requested amount.
            let market = token::Client::new(&env, &auction_data.settings.market);
            market.transfer(
                &buyer,
                &env.current_contract_address(),
                &auction_data.settings.sealed_bid_deposit,
            );
            auction_data.deposits.push_back(BidData {
                buyer: buyer.clone(),
                amount: auction_data.settings.sealed_bid_deposit,
                sniper: false,
            });
            env.events()
                .publish((BID, symbol_short!("sealed")), auction_id);
        }

        storage::set::<DataKey, AuctionData>(env, &DataKey::AuctionData(auction_id), &auction_data);
    }

    fn place_bid(
        &self,
        env: &Env,
        auction_id: u64,
        buyer: &Address,
        amount: i128,
        salt: &Option<BytesN<32>>,
    ) {
        // First check that the auction is resolved.
        let resolved = self.resolve(env, auction_id);
        if resolved {
            return;
        }

        let mut auction_data =
            storage::get::<DataKey, AuctionData>(env, &DataKey::AuctionData(auction_id)).unwrap();
        let market = token::Client::new(&env, &auction_data.settings.market);

        // Reveal the sealed bid.
        match self.is_sealed_bid_auction(&auction_data) {
            true => {
                // Reveal the sealed bid.
                let mut data = Bytes::from_array(&env, &amount.to_be_bytes());
                data.append(&Bytes::from_slice(&env, &salt.as_ref().unwrap().to_array()));
                data.append(&Bytes::from_slice(&env, &auction_data.id.to_be_bytes()));
                self.reveal_bid(env, &data);

                // Refund the deposit.
                if let Some(index) = auction_data.deposits.iter().position(|b| {
                    b.amount == auction_data.settings.sealed_bid_deposit && b.buyer == *buyer
                }) {
                    let bid = &auction_data.deposits.get_unchecked(index as u32);
                    market.transfer(&env.current_contract_address(), &buyer, &bid.amount);
                    auction_data.deposits.remove(index as u32);
                    env.events()
                        .publish((BID, symbol_short!("refunded")), auction_id);
                } else {
                    panic!("Invalid bid");
                }
            }
            false => { /* continue */ }
        }

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
                    .publish((BID, symbol_short!("deleted")), auction_id);
            } else {
                panic!("No bid to cancel");
            }
        } else if amount > 0 && amount >= auction_data.settings.reserve_price {
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
                    >= auction_data.start_time + auction_data.settings.duration - anti_snipe_time;
                if sniper {
                    auction_data.settings.duration += anti_snipe_time;
                }

                auction_data.bids.push_back(BidData {
                    buyer: buyer.clone(),
                    amount,
                    sniper,
                });
                env.events()
                    .publish((BID, symbol_short!("added")), auction_id);
            } else {
                panic!("Not allowed to place new bid");
            }
        } else {
            panic!("Invalid bid");
        }

        storage::set::<DataKey, AuctionData>(env, &DataKey::AuctionData(auction_id), &auction_data);
        self.resolve(env, auction_id);
    }

    fn finalize(&self, env: &Env, auction_id: u64, winner: Option<&BidData>) -> bool {
        let auction_data =
            storage::get::<DataKey, AuctionData>(env, &DataKey::AuctionData(auction_id)).unwrap();
        match winner {
            Some(bid) => {
                // We have a winner, transfer token to parties.
                let admin_data =
                    storage::get::<DataKey, AdminData>(&env, &DataKey::AdminData).unwrap();
                let token = token::Client::new(&env, &auction_data.settings.token);
                let market = token::Client::new(&env, &auction_data.settings.market);
                let admin: Address = admin_data.admin;
                let commission_rate: i128 = admin_data.commission_rate as i128;
                let admin_share = bid
                    .amount
                    .checked_mul(commission_rate)
                    .and_then(|val| val.checked_add(99))
                    .and_then(|val| val.checked_div(100))
                    .unwrap()
                    .max(1);
                let seller_share = bid.amount.checked_sub(admin_share).unwrap().max(1);

                token.transfer(
                    &env.current_contract_address(),
                    &bid.buyer,
                    &auction_data.settings.amount,
                );
                market.transfer(&env.current_contract_address(), &admin, &admin_share);
                market.transfer(
                    &env.current_contract_address(),
                    &auction_data.settings.seller,
                    &seller_share,
                );

                // Cancel all other bids.
                let market = token::Client::new(&env, &auction_data.settings.market);
                for b in auction_data.bids.iter() {
                    if b.amount > 0 && b.buyer != bid.buyer {
                        market.transfer(&env.current_contract_address(), &b.buyer, &b.amount);
                    }
                }

                let region = &AuctionRegion::Dispatcher(auction_id);
                let state_machine = StateMachine::<AuctionRegion, AuctionPhase>::new(
                    region,
                    fsm::StorageType::Instance,
                );
                state_machine.remove_state(&env);

                // Delete the auction.
                storage::remove::<DataKey, AuctionData>(env, &DataKey::AuctionData(auction_id));
                env.events()
                    .publish((AUCTION, symbol_short!("won")), auction_id);
                true
            }
            None => {
                // No winner.
                // Transfer token back to seller.
                let token = token::Client::new(&env, &auction_data.settings.token);
                token.transfer(
                    &env.current_contract_address(),
                    &auction_data.settings.seller,
                    &auction_data.settings.amount,
                );

                // Cancel all bids.
                let market = token::Client::new(&env, &auction_data.settings.market);
                for bid in auction_data.bids.iter() {
                    if bid.amount > 0 {
                        market.transfer(&env.current_contract_address(), &bid.buyer, &bid.amount);
                    }
                }

                let region = &AuctionRegion::Dispatcher(auction_id);
                let state_machine = StateMachine::<AuctionRegion, AuctionPhase>::new(
                    region,
                    fsm::StorageType::Instance,
                );
                state_machine.remove_state(&env);

                // Delete the auction.
                storage::remove::<DataKey, AuctionData>(env, &DataKey::AuctionData(auction_id));
                env.events()
                    .publish((AUCTION, symbol_short!("ended")), auction_id);
                true
            }
        }
    }

    fn is_sealed_bid_auction(&self, auction_data: &AuctionData) -> bool {
        auction_data.settings.sealed_bid_deposit > 0
            && auction_data.settings.sealed_phase_time > 0
            && auction_data.settings.discount_percent == 0
            && auction_data.settings.discount_frequency == 0
    }

    // Commit.
    #[commit(hash = "sealed")]
    fn commit_bid(&self, env: &Env, sealed: &BytesN<32>) {}

    // Reveal.
    #[reveal(data = "amount")]
    fn reveal_bid(&self, env: &Env, amount: &Bytes) {}

    fn resolve(&self, env: &Env, auction_id: u64) -> bool;

    fn calculate_price(&self, env: &Env, auction_id: u64) -> i128;
}

// The Dispatcher uses the `state-machine` to control auction phases
#[derive(TransitionHandler)]
pub enum Dispatcher {
    AscendingPriceAuction,
    DescendingPriceAuction,
}

impl BaseAuction for Dispatcher {
    fn start(&self, env: &Env, auction_id: u64, auction_data: &AuctionData) {
        match self {
            Dispatcher::AscendingPriceAuction => {
                AscendingPriceAuction.start(env, auction_id, auction_data)
            }
            Dispatcher::DescendingPriceAuction => {
                DescendingPriceAuction.start(env, auction_id, auction_data)
            }
        }
    }

    #[state_machine(
        state = "AuctionPhase:Committing",
        region = "AuctionRegion:Dispatcher:auction_id"
    )]
    fn place_sealed_bid(
        &self,
        env: &Env,
        auction_id: u64,
        buyer: &Address,
        sealed_amount: &BytesN<32>,
    ) {
        match self {
            Dispatcher::AscendingPriceAuction => {
                AscendingPriceAuction.place_sealed_bid(env, auction_id, buyer, sealed_amount)
            }
            Dispatcher::DescendingPriceAuction => {
                DescendingPriceAuction.place_sealed_bid(env, auction_id, buyer, sealed_amount)
            }
        }
    }

    #[state_machine(
        state = "AuctionPhase:Running",
        region = "AuctionRegion:Dispatcher:auction_id"
    )]
    fn place_bid(
        &self,
        env: &Env,
        auction_id: u64,
        buyer: &Address,
        amount: i128,
        salt: &Option<BytesN<32>>,
    ) {
        match self {
            Dispatcher::AscendingPriceAuction => {
                AscendingPriceAuction.place_bid(env, auction_id, buyer, amount, salt)
            }
            Dispatcher::DescendingPriceAuction => {
                DescendingPriceAuction.place_bid(env, auction_id, buyer, amount, salt)
            }
        }
    }

    fn resolve(&self, env: &Env, auction_id: u64) -> bool {
        match self {
            Dispatcher::AscendingPriceAuction => AscendingPriceAuction.resolve(env, auction_id),
            Dispatcher::DescendingPriceAuction => DescendingPriceAuction.resolve(env, auction_id),
        }
    }

    fn calculate_price(&self, env: &Env, auction_id: u64) -> i128 {
        match self {
            Dispatcher::AscendingPriceAuction => {
                AscendingPriceAuction.calculate_price(env, auction_id)
            }
            Dispatcher::DescendingPriceAuction => {
                DescendingPriceAuction.calculate_price(env, auction_id)
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
