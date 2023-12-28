/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use crate::types::{License, MarketData, MarketDataKey};
use soroban_kit::storage;
use soroban_sdk::{vec, Address, Env, Vec};

pub struct CompensationPercentage;

// Percentage royalty payment.
impl super::r#impl::Agreement for CompensationPercentage {
    fn calculate_interest(&self, env: &Env, license: &License) -> Vec<(i128, Address)> {
        // Fed to contract from oracle broker.
        let data = storage::get::<MarketDataKey, MarketData>(
            &env,
            &MarketDataKey::Index(license.terms.property.clone()),
        )
        .unwrap();
        let mut interest = vec![
            env,
            (
                data.price
                    .checked_mul(license.terms.royalty_interest)
                    .and_then(|val| val.checked_add(99))
                    .and_then(|val| val.checked_div(100))
                    .unwrap(),
                data.asset,
            ),
        ];
        if license.transferring && license.terms.transfer_fee > 0 {
            interest.push_back((license.terms.transfer_fee, license.terms.currency.clone()));
        }
        interest
    }
}
