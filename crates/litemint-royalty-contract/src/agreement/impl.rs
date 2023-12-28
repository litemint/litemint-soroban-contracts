/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

use crate::types::{AdminData, AdminDataKey};
use crate::types::{Compensation, License, LicenseStatus};
use soroban_kit::storage;
use soroban_sdk::{token, Address, Env, Vec};

use super::compensation_fixed::*;
use super::compensation_percentage::*;
use super::compensation_subscription::*;

pub trait Agreement {
    fn execute(&self, env: &Env, license: &mut License) {
        #[cfg(not(test))]
        let require_enforcement = |env: &Env, license: &License, licensor_balance: i128| -> bool {
            license.grace_time < env.ledger().timestamp() && licensor_balance == 0
        };

        #[cfg(test)]
        let require_enforcement = |_env: &Env, license: &License, licensor_balance: i128| -> bool {
            license.grace_time == 1473 && licensor_balance == 0 /* TEST:LATE */
        };

        #[cfg(not(test))]
        let has_recur_elapsed =
            |_env: &Env, license: &License, now: u64, licensor_balance: i128| -> bool {
                license.recur_time > 0 && license.recur_time < now && licensor_balance == 0
            };

        #[cfg(test)]
        let has_recur_elapsed =
            |_env: &Env, license: &mut License, _now: u64, licensor_balance: i128| -> bool {
                match licensor_balance != 0 {
                    true => false,
                    false => {
                        let result = license.recur_time == 707; // TEST:RECUR
                        license.recur_time += 1;
                        result
                    }
                }
            };

        let licensor_balance =
            token::Client::new(&env, &license.terms.property).balance(&license.terms.licensor);

        match license.status {
            LicenseStatus::Paid => {
                let now = env.ledger().timestamp();
                let property = token::Client::new(env, &license.terms.property);
                // Licensor holding.
                if licensor_balance > 0 {
                    license.licensee = license.terms.licensor.clone();
                }
                // Ownership has changed, payment due.
                else if property.balance(&license.licensee) == 0 {
                    license.status = LicenseStatus::Unpaid;
                    license.grace_time = now + license.terms.grace_period;
                    license.transferring = true;
                // Recurring period elapsed, payment due.
                } else if has_recur_elapsed(env, license, now, licensor_balance) {
                    license.status = LicenseStatus::Unpaid;
                    match license.terms.licensor == license.licensee {
                        true => license.recur_time = now + license.terms.recur_period,
                        false => license.recur_time += license.terms.recur_period,
                    }
                    license.grace_time = (now + license.terms.grace_period).min(license.recur_time);
                }
            }
            LicenseStatus::Unpaid if require_enforcement(env, license, licensor_balance) => {
                // Successful interest calculation is required to guarantee symmetry with payments.
                self.calculate_interest(&env, &license);

                // Send the lien to licensor so they can seize the property.
                token::Client::new(env, &license.terms.lien).transfer(
                    &env.current_contract_address(),
                    &license.terms.licensor,
                    &1,
                );
                license.status = LicenseStatus::Breached;
            }
            _ => {}
        }
    }

    fn pay(&self, env: &Env, new_licensee: &Address, license: &mut License) {
        self.execute(env, license);

        if license.status == LicenseStatus::Unpaid {
            let interest = self.calculate_interest(env, license);
            for (amount, market) in interest {
                let payment_token = token::Client::new(env, &market);
                let admin_data =
                    storage::get::<AdminDataKey, AdminData>(env, &AdminDataKey::Root).unwrap();
                let (admin_share, licensor_share) = if admin_data.commission_rate > 0 {
                    let admin_share = amount
                        .checked_mul(admin_data.commission_rate)
                        .and_then(|val| val.checked_add(99))
                        .and_then(|val| val.checked_div(100))
                        .unwrap()
                        .max(1);

                    let licensor_share = (amount - admin_share).max(1);
                    (admin_share, licensor_share)
                } else {
                    // No commission, all goes to licensor.
                    (0, amount)
                };

                if admin_share > 0 {
                    payment_token.transfer(
                        new_licensee,
                        &env.current_contract_address(),
                        &admin_share,
                    );
                }
                payment_token.transfer(new_licensee, &license.terms.licensor, &licensor_share);
            }

            license.transferring = false;
            license.status = LicenseStatus::Paid;
            license.licensee = new_licensee.clone();
        }

        self.execute(env, license);
    }

    fn calculate_interest(&self, env: &Env, license: &License) -> Vec<(i128, Address)>;
}

impl Agreement for Compensation {
    fn pay(&self, env: &Env, new_licensee: &Address, license: &mut License) {
        match self {
            Compensation::Fixed => CompensationFixed.pay(env, new_licensee, license),
            Compensation::Percentage => CompensationPercentage.pay(env, new_licensee, license),
            Compensation::Subscription => CompensationSubscription.pay(env, new_licensee, license),
        }
    }

    fn execute(&self, env: &Env, license: &mut License) {
        match self {
            Compensation::Fixed => CompensationFixed.execute(env, license),
            Compensation::Percentage => CompensationPercentage.execute(env, license),
            Compensation::Subscription => CompensationSubscription.execute(env, license),
        }
    }

    fn calculate_interest(&self, env: &Env, license: &License) -> Vec<(i128, Address)> {
        match self {
            Compensation::Fixed => CompensationFixed.calculate_interest(env, license),
            Compensation::Percentage => CompensationPercentage.calculate_interest(env, license),
            Compensation::Subscription => CompensationSubscription.calculate_interest(env, license),
        }
    }
}

#[macro_export]
macro_rules! agreement {
    ($type:expr) => {
        match $type {
            Compensation::Fixed => Compensation::Fixed,
            Compensation::Percentage => Compensation::Percentage,
            Compensation::Subscription => Compensation::Subscription,
        }
    };
}
