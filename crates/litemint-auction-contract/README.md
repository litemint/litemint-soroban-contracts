[![MIT License][license-shield]][license-url]
[![Twitter][twitter-shield]][twitter-url]

# litemint-auction-contract
![Build Status](https://github.com/litemint/litemint-soroban-contracts/actions/workflows/rust.yml/badge.svg)
[![litemint-auction-contract version](https://img.shields.io/crates/v/litemint-auction-contract.svg)](https://crates.io/crates/litemint-auction-contract)

Litemint auction smart contract powering the Litemint marketplace.

Licensed under MIT. This software is provided "AS IS", no liability assumed. [More details](LICENSE).

- [litemint-auction-contract](#litemint-auction-contract)
  - [Introduction](#introduction)
  - [Dependencies](#dependencies)
      - [soroban-kit](#soroban-kit)
  - [Getting Started](#getting-started)
  - [Contributing](#contributing)
  - [License](#license)
  - [Contact](#contact)

## Introduction

Since 2021, the Litemint marketplace has utilized the Stellar DEX for time-based auctions, leveraging time-bound, pre-auth transactions [details in our blog](https://blog.litemint.com/anatomy-of-a-stellar-powered-auction-on-litemint/). While these auctions offer security and interoperability, they lack flexibilities, such as anti-snipe mechanisms and varied bidding strategies like descending auctions.

The Litemint Auction Contract on [Soroban](https://soroban.stellar.org) (Stellar's Rust-based smart contracts platform), addresses these limitations. The smart contract enhances the Litemint marketplace while co-existing with our SDEX-based method, offering users a comprehensive and versatile auction experience.

This contract implements a range of features, including:

- [X] Time-based auctions with decentralized resolution.
- [X] Sealed bid auctions.
- [X] Descending price auctions (see [behavior_descending_price.rs](src/auctions/behavior_descending_price.rs)) supporting linear or compound discount, and customizable frequency/rate.
- [X] Ascending price auctions (see [behavior_ascending_price.rs](src/auctions/behavior_ascending_price.rs)) with "**_buy now_**" option.
- [X] Support for `reserve price` and `ask price`.
- [X] Anti-snipe mechanism. Auction sniping automatically increases the auction duration (time configurable by admin) and prevents the sniper to either cancel or submit a new bid.
- [X] Configurable marketplace commission rate.
- [X] Extendable auction duration by seller.
- [X] Support for concurrent and cancellable bids.
- [X] Strategy design pattern for easily adding new auction behaviors.

## Dependencies

#### soroban-kit
  
  `soroban-kit` provides fast, lightweight functions and macros with lean, targeted functionality for Soroban smart contract development:
  [https://github.com/FredericRezeau/soroban-kit](https://github.com/FredericRezeau/soroban-kit).

  The Litemint auction contract uses the following features from `soroban-kit`:
  - [X] `commitment-scheme` to implement sealed bid auctions.
  - [X] `state-machine` to manage auction phases.
  - [X] `storage` for type safety with storage operations.

## Getting Started

From the workspace root:

1. Cloning the repository:
   ```sh
   git clone https://github.com/Litemint/litemint-soroban-contracts.git
   ```
2. Building the contracts:
   ```sh
   soroban contract build
   ```
3. Running Tests:
   ```sh
   cargo test -- --nocapture
   ```
4. Deploying to testnet:
   ```sh
   soroban contract deploy --wasm target/wasm32-unknown-unknown/release/litemint_auction_contract.wasm --source ACCOUNT --rpc-url https://soroban-testnet.stellar.org:443 --network-passphrase "Test SDF Network ; September 2015"
   ```
   ```sh
   output > CONTRACT_ID
   ```
5. Initialize admin:
   ```sh
   soroban contract invoke --id CONTRACT_ID --source ACCOUNT --rpc-url https://soroban-testnet.stellar.org:443 --network-passphrase "Test SDF Network ; September 2015" -- initialize --admin ACCOUNT --anti_snipe_time 60 --commission_rate 5 --extendable_auctions true
   ```

## Contributing

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/feature`)
3. Commit your Changes (`git commit -m 'Add some feature'`)
4. Push to the Branch (`git push origin feature/feature`)
5. Open a Pull Request

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for more information.

## Contact

LitemintHQ on X - [@LitemintHQ](https://twitter.com/LitemintHQ)

Litemint Marketplace: [https://litemint.com](https://litemint.com)

Join our discord server: [https://litemint.gg](https://litemint.gg)

[license-shield]: https://img.shields.io/github/license/litemint/litemint-soroban-contracts.svg?style=for-the-badge
[license-url]: https://github.com/litemint/litemint-soroban-contracts/blob/master/LICENSE
[twitter-shield]: https://img.shields.io/badge/-Twitter-black.svg?style=for-the-badge&logo=twitter&colorB=555
[twitter-url]: https://x.com/liteminthq

[rust-shield]: https://img.shields.io/badge/Rust-000000?style=flat-square&logo=Rust&logoColor=white
[rust-url]: https://www.rust-lang.org
[javascript-shield]: https://img.shields.io/badge/JavaScript-F7DF1E?style=flat-square&logo=javascript&logoColor=black
[javascript-url]: https://vanilla-js.com
