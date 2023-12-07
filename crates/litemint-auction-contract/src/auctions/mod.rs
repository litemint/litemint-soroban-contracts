/*
    Date: 2023
    Author: Fred Kyung-jin Rezeau <fred@litemint.com>
    Copyright (c) 2023 Litemint LLC

    MIT License
*/

//! The `auctions` module implements a time-based auction system using trait-based polymorphism
//! and enum-based dispatch (strategy design pattern) to allow modular extension
//! for auction behaviors.
//!
//! Implemented features:
//!
//! - Descending price auctions (see: behavior_descending_price.rs) supporting linear
//!   or compound discount, and customizable frequency/rate.
//! - Ascending price auctions (see: behavior_ascending_price.rs) with "buy now" option.
//! - Reserve price.
//! - Anti-snipe mechanism.
//! - Concurrent and cancellable bids.

pub mod behavior;
pub mod behavior_ascending_price;
pub mod behavior_descending_price;
