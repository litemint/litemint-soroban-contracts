/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

//! Learn more about Litemint timed auctions:
//! https://blog.litemint.com/anatomy-of-a-stellar-powered-auction-on-litemint/

#![no_std]

mod auctions; // Auction behaviors and mechanisms.
mod types; // Contract types.

use soroban_kit::{
    fsm::{self, StateMachine},
    storage,
};
use soroban_sdk::{contract, contractimpl, contractmeta, vec, Address, BytesN, Env, Vec};

use crate::auctions::{behavior::BaseAuction, behavior::Dispatcher};
use types::{AdminData, AuctionData, AuctionPhase, AuctionRegion, AuctionSettings, DataKey};

contractmeta!(
    key = "desc",
    val = "Auction smart contract for the Litemint marketplace"
);

pub trait AuctionContractTrait {
    // Upgrade this contract.
    // Admin authorization required.
    fn upgrade(e: Env, wasm_hash: BytesN<32>);

    // Retrieves auction data for an existing auction.
    // No authorization required.
    fn get_auction(env: Env, auction_id: u64) -> Option<AuctionData>;

    // Resolves the auction.
    // No authorization required.
    fn resolve(env: Env, auction_id: u64);

    // Place a sealed bid.
    // Require auction to be in `commit` phase.
    // Bid amount must be sealed using `sha256([big_endian_amount;16][salt;32][big_endian_auction_id;8])`.
    // Buyer authorization required.
    fn place_sealed_bid(env: Env, auction_id: u64, buyer: Address, sealed_amount: BytesN<32>);

    // Place or reveal a bid.
    // Late bids (i.e., within anti_snipe_time from the end of the auction)
    // are subject to anti-snipe rules and cannot be cancelled or modified.
    // Buyer authorization required.
    fn place_bid(env: Env, auction_id: u64, buyer: Address, amount: i128, salt: Option<BytesN<32>>);

    // Extend the duration of an ongoing auction.
    // Require admin settings `extendable_auctions` set to true.
    // Seller authorization required.
    fn extend(env: Env, auction_id: u64, duration: u64) -> bool;

    // Start a new auction.
    // Return the new `auction_id`.
    // - Behaves as descending price auction if both `discount_percent` and `discount_frequency` have non-zero values.
    // - Enters `commit` phase for sealed bids if both `sealed_phase_time` and `sealed_bid_deposit` are specified.
    // Notes:
    // - When in `running` phase, the auction can be instantly won if a bidder meets or exceeds the `ask_price`,
    //   provided it is set above the `reserve_price` or discounted below the bid amount for descending auctions.
    // - `discount_percent` and `discount_frequency` are ignored for sealed bid auctions.
    // Seller authorization required.
    fn start(env: Env, auction_settings: AuctionSettings) -> u64;

    // Contract administration.
    // Admin authorization required.
    fn initialize(
        env: Env,
        admin: Address,
        anti_snipe_time: u64,
        commission_rate: i128,
        extendable_auctions: bool,
    );

    // Retrieve the contract version.
    fn version(env: Env) -> Vec<u32>;
}

#[contract]
struct AuctionContract;

#[contractimpl]
impl AuctionContractTrait for AuctionContract {
    fn upgrade(env: Env, wasm_hash: BytesN<32>) {
        storage::get_or_else::<DataKey, AdminData, _, _>(&env, &DataKey::AdminData, |opt| {
            opt.unwrap_or_else(|| panic!("Admin not set"))
        })
        .admin
        .require_auth();
        env.deployer().update_current_contract_wasm(wasm_hash);
    }

    fn get_auction(env: Env, auction_id: u64) -> Option<AuctionData> {
        storage::get_or_else::<DataKey, AuctionData, _, _>(
            &env,
            &DataKey::AuctionData(auction_id),
            |opt| opt,
        )
    }

    fn resolve(env: Env, auction_id: u64) {
        let auction_data =
            storage::get::<DataKey, AuctionData>(&env, &DataKey::AuctionData(auction_id)).unwrap();
        dispatcher!(
            auction_data.settings.discount_percent > 0
                && auction_data.settings.discount_frequency > 0
        )
        .resolve(&env, auction_id);
    }

    fn place_bid(
        env: Env,
        auction_id: u64,
        buyer: Address,
        amount: i128,
        salt: Option<BytesN<32>>,
    ) {
        buyer.require_auth();

        let auction_data =
            storage::get::<DataKey, AuctionData>(&env, &DataKey::AuctionData(auction_id)).unwrap();

        let dispatcher = dispatcher!(
            auction_data.settings.discount_percent > 0
                && auction_data.settings.discount_frequency > 0
        );

        #[cfg(test)]
        fn has_sealed_phase_expired(_env: &Env, _auction_data: &AuctionData) -> bool {
            true
        }

        #[cfg(not(test))]
        fn has_sealed_phase_expired(env: &Env, auction_data: &AuctionData) -> bool {
            auction_data.start_time + auction_data.settings.sealed_phase_time
                <= env.ledger().timestamp()
        }

        if dispatcher.is_sealed_bid_auction(&auction_data) {
            let region = AuctionRegion::Dispatcher(auction_id);
            if has_sealed_phase_expired(&env, &auction_data) {
                let state_machine = StateMachine::<AuctionRegion, AuctionPhase>::new(
                    &region,
                    fsm::StorageType::Instance,
                );
                state_machine.set_state(&env, &AuctionPhase::Running);
            }
        }

        dispatcher.place_bid(&env, auction_id, &buyer, amount, &salt);
    }

    fn place_sealed_bid(env: Env, auction_id: u64, buyer: Address, sealed_amount: BytesN<32>) {
        buyer.require_auth();

        let auction_data =
            storage::get::<DataKey, AuctionData>(&env, &DataKey::AuctionData(auction_id)).unwrap();
        dispatcher!(
            auction_data.settings.discount_percent > 0
                && auction_data.settings.discount_frequency > 0
        )
        .place_sealed_bid(&env, auction_id, &buyer, &sealed_amount);
    }

    fn extend(env: Env, auction_id: u64, duration: u64) -> bool {
        if !storage::get_or_else::<DataKey, AdminData, _, _>(&env, &DataKey::AdminData, |opt| {
            opt.unwrap_or_else(|| panic!("Admin not set"))
        })
        .extendable_auctions
        {
            false
        } else {
            let mut auction_data =
                storage::get::<DataKey, AuctionData>(&env, &DataKey::AuctionData(auction_id))
                    .unwrap();
            auction_data.settings.seller.require_auth();
            auction_data.settings.duration += duration;
            storage::set::<DataKey, AuctionData>(
                &env,
                &DataKey::AuctionData(auction_id),
                &auction_data,
            );
            true
        }
    }

    fn start(env: Env, auction_settings: AuctionSettings) -> u64 {
        if !storage::has::<DataKey, AdminData>(&env, &DataKey::AdminData) {
            panic!("Admin not set");
        }

        auction_settings.seller.require_auth();

        let mut id = 0u64;
        env.prng().fill(&mut id);
        let auction_data = AuctionData::new(
            auction_settings,
            env.ledger().timestamp(),
            vec![&env],
            vec![&env],
            id,
        );
        dispatcher!(
            auction_data.settings.discount_percent > 0
                && auction_data.settings.discount_frequency > 0
        )
        .start(&env, id, &auction_data);
        id
    }

    fn initialize(
        env: Env,
        admin: Address,
        anti_snipe_time: u64,
        commission_rate: i128,
        extendable_auctions: bool,
    ) {
        if storage::has::<DataKey, AdminData>(&env, &DataKey::AdminData) {
            panic!("Admin already set");
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

    fn version(env: Env) -> Vec<u32> {
        vec![&env, 0, 1, 3] // "0.1.3"
    }
}

#[cfg(test)]
mod test;
