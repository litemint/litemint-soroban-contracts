[![MIT License][license-shield]][license-url]
[![Twitter][twitter-shield]][twitter-url]

# litemint-royalty-contract
![Build Status](https://github.com/litemint/litemint-soroban-contracts/actions/workflows/rust.yml/badge.svg)
[![litemint-auction-contract version](https://img.shields.io/crates/v/litemint-royalty-contract.svg)](https://crates.io/crates/litemint-royalty-contract)

Litemint royalty smart contract powering the Litemint marketplace.

Licensed under MIT. This software is provided "AS IS", no liability assumed. [More details](LICENSE).

- [litemint-royalty-contract](#litemint-royalty-contract)
  - [Introduction](#introduction)
  - [Feature List](#feature-list)
  - [Dependencies](#dependencies)
      - [soroban-kit](#soroban-kit)
  - [Getting Started](#getting-started)
  - [Contributing](#contributing)
  - [License](#license)
  - [Contact](#contact)

## Introduction

Royalties play a pivotal role in digital economies.

The industry has encountered numerous obstacles in achieving decentralized payment enforcements (we discussed this topic at Meridian 2022, see [video excerpt here](https://twitter.com/LitemintHQ/status/1581565573112401925)). Currently, most marketplaces retain significant control over enforcing royalty payments (see [this Tweet](https://twitter.com/opensea/status/1626682043655507969) from OpenSea), which poses challenges to creators.

To address these challenges, we have identified a unique combination with Soroban smart contracts, oracles, and Stellar classic primitives (pre-auth transactions) allowing us to implement an unobtrusive solution for on-chain NFT royalty payment enforcements.

The Litemint royalty contract implements multiple royalty payment schemes for non-fungible tokens, including fixed, subscription, and percentage-based models. A key feature is its ability to enforce royalty payments without *isolating* NFTs from the Stellar DEX. Our approach ensures that NFT creators and collectors can freely hold and trade their NFTs from any Stellar DEX compatible service, enjoying an unrestricted sales funnel. 

## Feature List

- [X] Percentage-based royalty payments (see [compensation_percentage.rs](https://github.com/litemint/litemint-soroban-contracts/src/agreement/compensation_percentage.rs)).
- [X] Fixed royalty payments (see [compensation_fixed.rs](https://github.com/litemint/litemint-soroban-contracts/src/agreement/compensation_fixed.rs)).
- [X] Subcription royalty payments (see [compensation_subscription.rs](https://github.com/litemint/litemint-soroban-contracts/src/agreement/compensation_subscription.rs)).
- [X] Decentralized on-chain payment enforcement.
- [X] NFTs compatibility with all ecosystem services.
- [X] Support for all currencies and markets.
- [X] Optional license transfer fee.
- [X] Configurable grace period and marketplace commission rate.
- [X] Strategy design pattern for easily adding new royalty schemes.

## Dependencies

#### soroban-kit
  
  `soroban-kit` provides fast, lightweight functions and macros with lean, targeted functionality for Soroban smart contract development:
  [https://github.com/FredericRezeau/soroban-kit](https://github.com/FredericRezeau/soroban-kit).

  The Litemint royalty contract uses the following features from `soroban-kit`:
  - [X] `oracles` to receive external market data feed.
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
