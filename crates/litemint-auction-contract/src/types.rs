/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use soroban_kit::{key_constraint, soroban_tools, storage};
use soroban_sdk::{contracttype, Address, Env, Vec};

#[derive(Clone)]
#[contracttype]
#[key_constraint(DataKeyConstraint)]
pub enum DataKey {
    AdminData,
    AuctionData(u64),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuctionRegion {
    Dispatcher(u64),
    Resolve(u64),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuctionPhase {
    Committing,
    Running,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BidData {
    pub buyer: Address,
    pub amount: i128,
    pub sniper: bool,
}

#[contracttype]
#[storage(Instance, DataKeyConstraint)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminData {
    pub admin: Address,
    pub anti_snipe_time: u64,
    pub commission_rate: i128,
    pub extendable_auctions: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuctionSettings {
    pub seller: Address,
    pub token: Address,
    pub amount: i128,
    pub duration: u64,
    pub market: Address,
    pub reserve_price: i128,
    pub ask_price: i128,
    pub discount_percent: u32,
    pub discount_frequency: u64,
    pub compounded_discount: bool,
    pub sealed_phase_time: u64,
    pub sealed_bid_deposit: i128,
}

#[contracttype]
#[storage(Persistent, DataKeyConstraint)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuctionData {
    pub settings: AuctionSettings,
    pub start_time: u64,
    pub bids: Vec<BidData>,
    pub deposits: Vec<BidData>,
    pub id: u64,
}

impl AuctionData {
    pub fn new(
        settings: AuctionSettings,
        start_time: u64,
        bids: Vec<BidData>,
        deposits: Vec<BidData>,
        id: u64,
    ) -> Self {
        AuctionData {
            settings,
            start_time,
            bids,
            deposits,
            id,
        }
    }
}
