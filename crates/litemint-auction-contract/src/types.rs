/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use soroban_macros::{key_constraint, storage};
use soroban_sdk::{contracttype, Address, Env, Vec};

#[derive(Clone)]
#[contracttype]
#[key_constraint(DataKeyConstraint)]
pub enum DataKey {
    AdminData,
    AuctionData(Address),
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
#[storage(Persistent, DataKeyConstraint)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuctionData {
    pub token: Address,
    pub amount: i128,
    pub duration: u64,
    pub start_time: u64,
    pub market: Address,
    pub reserve_price: i128,
    pub ask_price: i128,
    pub discount_percent: u32,
    pub discount_frequency: u64,
    pub compounded_discount: bool,
    pub bids: Vec<BidData>,
}
