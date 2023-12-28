/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use soroban_kit::{key_constraint, soroban_tools, storage};
use soroban_sdk::{contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
#[key_constraint(DataKeyConstraint)]
pub(crate) enum DataKey {
    License(Address),
    BrokerWhitelist(Address),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Compensation {
    Fixed,
    Percentage,
    Subscription,
}

impl Compensation {
    pub fn from_u64(value: u64) -> Option<Self> {
        match value {
            1 => Some(Compensation::Percentage),
            2 => Some(Compensation::Subscription),
            _ => Some(Compensation::Fixed),
        }
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Terms {
    pub licensor: Address,
    pub property: Address,
    pub lien: Address,
    pub compensation: Compensation,
    pub royalty_interest: i128,
    pub transfer_fee: i128,
    pub currency: Address,
    pub recur_period: u64,
    pub grace_period: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LicenseStatus {
    Unpaid,
    Paid,
    Breached,
}

#[contracttype]
#[storage(Instance, DataKeyConstraint)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct License {
    pub terms: Terms,
    pub licensee: Address,
    pub created_time: u64,
    pub recur_time: u64,
    pub grace_time: u64,
    pub status: LicenseStatus,
    pub transferring: bool,
}

impl License {
    pub fn new(
        terms: Terms,
        licensee: Address,
        created_time: u64,
        recur_time: u64,
        grace_time: u64,
        status: LicenseStatus,
        transferring: bool,
    ) -> Self {
        License {
            terms,
            licensee,
            created_time,
            recur_time,
            grace_time,
            status,
            transferring,
        }
    }
}

#[derive(Clone)]
#[contracttype]
#[key_constraint(AdminDataKeyConstraint)]
pub(crate) enum AdminDataKey {
    Root,
}

#[contracttype]
#[storage(Instance, AdminDataKeyConstraint)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AdminData {
    pub admin: Address,
    pub commission_rate: i128,
}

#[contracttype]
#[key_constraint(MarketDataKeyConstraint)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum MarketDataKey {
    Index(Address),
}

#[contracttype]
#[storage(Instance, MarketDataKeyConstraint)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketData {
    pub price: i128,
    pub asset: Address,
}
