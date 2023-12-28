/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

#![no_std]

mod agreement; // Agreement model and mechanisms.
mod types;

use soroban_kit::{oracle, oracle_subscriber, storage};
use soroban_sdk::{contract, contractimpl, contractmeta, token, Address, Env};
use types::{MarketData, MarketDataKey};

use crate::{
    agreement::Agreement,
    types::{AdminData, AdminDataKey, Compensation, DataKey, License, LicenseStatus, Terms},
};

contractmeta!(
    key = "desc",
    val = "NFT royalty smart contract for the Litemint marketplace"
);

pub trait RoyaltyInterface {
    // Execute the royalty agreement for a property. Can be called at anytime, by anyone.
    // No authorization required.
    fn execute(env: Env, property: Address) -> License;

    // Pay the royalty for a property (e.g., after transfer of ownership, recurring subscription...).
    // Licensee authorization required.
    fn pay(env: Env, property: Address, licensee: Address) -> License;

    // Add a property to the contract.
    // Licensor authorization required.
    fn add_property(env: Env, terms: Terms);
}

pub trait Subscriber {
    // Allow an oracle broker to publish price feeds to this contract.
    // Admin authorization required.
    fn allow_broker(env: Env, broker: Address);

    // Deny an oracle broker.
    // Admin authorization required.
    fn deny_broker(env: Env, broker: Address);
}

#[contract]
#[oracle_subscriber(Address, MarketData)]
struct RoyaltyContract;

// Implement the Oracle events.
impl oracle::Events<Address, MarketData> for RoyaltyContract {
    fn on_request(env: &Env, _topic: &Address, envelope: &oracle::Envelope) {
        require_broker_whitelisted(env, &envelope.broker);
        envelope.subscriber.require_auth();
    }

    fn on_sync_receive(env: &Env, topic: &Address, envelope: &oracle::Envelope, data: &MarketData) {
        require_broker_whitelisted(env, &envelope.broker);
        storage::set::<MarketDataKey, MarketData>(
            &env,
            &MarketDataKey::Index(topic.clone()),
            reconcile_data(&mut data.clone()),
        );
    }

    fn on_async_receive(
        env: &Env,
        topic: &Address,
        envelope: &oracle::Envelope,
        data: &MarketData,
    ) {
        require_broker_whitelisted(env, &envelope.broker);
        // Make sure this cross-contract call is from broker.
        envelope.broker.require_auth();
        storage::set::<MarketDataKey, MarketData>(
            &env,
            &MarketDataKey::Index(topic.clone()),
            reconcile_data(&mut data.clone()),
        );
    }
}

fn reconcile_data<'a>(data: &'a mut MarketData) -> &'a mut MarketData {
    // We might implement more sophisticated data reconciliation strategies to derive
    // additional compensation models (e.g., average...).

    // For now, replacing the last price is enough to serve our current
    // compensation schemes.
    data
}

#[contractimpl]
impl Subscriber for RoyaltyContract {
    fn allow_broker(env: Env, broker: Address) {
        storage::get::<AdminDataKey, AdminData>(&env, &AdminDataKey::Root)
            .unwrap()
            .admin
            .require_auth();
        update_broker_whitelist(&env, &broker, false);
    }

    fn deny_broker(env: Env, broker: Address) {
        storage::get::<AdminDataKey, AdminData>(&env, &AdminDataKey::Root)
            .unwrap()
            .admin
            .require_auth();
        update_broker_whitelist(&env, &broker, true);
    }
}

#[contractimpl]
impl RoyaltyInterface for RoyaltyContract {
    fn execute(env: Env, property: Address) -> License {
        let mut license =
            storage::get::<DataKey, License>(&env, &DataKey::License(property.clone())).unwrap();
        agreement!(license.terms.compensation).execute(&env, &mut license);
        storage::set::<DataKey, License>(&env, &DataKey::License(property), &license);
        license
    }

    fn pay(env: Env, property: Address, licensee: Address) -> License {
        licensee.require_auth();

        let mut license =
            storage::get::<DataKey, License>(&env, &DataKey::License(property.clone())).unwrap();
        assert_eq!(
            token::Client::new(&env, &license.terms.property).balance(&licensee),
            1
        );
        assert_eq!(
            token::Client::new(&env, &license.terms.lien).balance(&env.current_contract_address()),
            1
        );

        agreement!(license.terms.compensation).pay(&env, &licensee, &mut license);
        storage::set::<DataKey, License>(&env, &DataKey::License(property), &license);
        license
    }

    fn add_property(env: Env, terms: Terms) {
        terms.licensor.require_auth();

        let property = terms.property.clone();
        assert!(terms.recur_period > terms.grace_period || terms.recur_period == 0);
        assert!(!storage::has::<DataKey, License>(
            &env,
            &DataKey::License(property.clone())
        ));
        assert_eq!(
            token::Client::new(&env, &property).balance(&terms.licensor),
            1
        );

        // Transfer the lien to contract.
        token::Client::new(&env, &terms.lien).transfer(
            &terms.licensor,
            &env.current_contract_address(),
            &1,
        );

        // Create the license.
        let licensee = terms.licensor.clone();
        let created_time = env.ledger().timestamp();
        let recur_time = if terms.recur_period > 0 {
            created_time + terms.recur_period
        } else {
            0
        };
        let grace_time = created_time + terms.grace_period;
        let license = License::new(
            terms,
            licensee,
            created_time,
            recur_time,
            grace_time,
            LicenseStatus::Paid,
            false,
        );
        storage::set::<DataKey, License>(&env, &DataKey::License(property), &license);
    }
}

#[contractimpl]
impl RoyaltyContract {
    pub fn initialize(env: Env, admin: Address, commission_rate: i128) {
        assert!(!storage::has::<AdminDataKey, AdminData>(
            &env,
            &AdminDataKey::Root
        ));
        storage::set::<AdminDataKey, AdminData>(
            &env,
            &AdminDataKey::Root,
            &AdminData {
                admin,
                commission_rate,
            },
        );
    }

    #[cfg(test)]
    pub fn test_oracle_feed(env: Env, topic: Address, price: i128, asset: Address) {
        storage::set::<MarketDataKey, MarketData>(
            &env,
            &MarketDataKey::Index(topic.clone()),
            reconcile_data(&mut MarketData { price, asset }),
        );
    }
}

fn update_broker_whitelist(env: &Env, broker: &Address, remove: bool) {
    match remove {
        true => env
            .storage()
            .instance()
            .remove::<DataKey>(&DataKey::BrokerWhitelist(broker.clone())),
        false => env
            .storage()
            .instance()
            .set::<DataKey, bool>(&DataKey::BrokerWhitelist(broker.clone()), &true),
    }
}

fn require_broker_whitelisted(env: &Env, broker: &Address) -> bool {
    env.storage()
        .instance()
        .get::<DataKey, bool>(&DataKey::BrokerWhitelist(broker.clone()))
        .unwrap()
}

#[cfg(test)]
mod test;
