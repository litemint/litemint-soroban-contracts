/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use crate::types::License;
use soroban_sdk::{vec, Address, Env, Vec};

pub struct CompensationSubscription;

// Recurring royalty payment.
impl super::r#impl::Agreement for CompensationSubscription {
    fn calculate_interest(&self, env: &Env, license: &License) -> Vec<(i128, Address)> {
        let mut interest = vec![
            env,
            (
                license.terms.royalty_interest,
                license.terms.currency.clone(),
            ),
        ];
        if license.transferring && license.terms.transfer_fee > 0 {
            interest.push_back((license.terms.transfer_fee, license.terms.currency.clone()));
        }
        interest
    }
}
