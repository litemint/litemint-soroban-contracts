[![MIT License][license-shield]][license-url]
[![Twitter][twitter-shield]][twitter-url]

# litemint-soroban-contracts

![Build Status](https://github.com/litemint/litemint-soroban-contracts/actions/workflows/rust.yml/badge.svg)

Official repo hosting the open source code of Litemint smart contracts powering the Litemint marketplace and games.

Licensed under MIT. This software is provided "AS IS", no liability assumed. [More details](LICENSE).

- [litemint-soroban-contracts](#litemint-soroban-contracts)
  - [Getting Started](#getting-started)
    - [Dependencies](#dependencies)
      - [soroban-kit](#soroban-kit)
    - [Running tests and building](#running-tests-and-building)
  - [Smart contracts](#smart-contracts)
    - [litemint-auction-contract](#litemint-auction-contract)
    - [litemint-royalty-contract](#litemint-royalty-contract)
  - [Contributing](#contributing)
  - [License](#license)
  - [Contact](#contact)

## Getting Started

### Dependencies

#### soroban-kit
  
  `soroban-kit` provides fast, lightweight functions and macros with lean, targeted functionality for Soroban smart contract development:
  [https://github.com/FredericRezeau/soroban-kit](https://github.com/FredericRezeau/soroban-kit).

Litemint smart contracts use the following features from `soroban-kit`:
  - [X] `commitment-scheme` to implement sealed bid auctions.
  - [X] `state-machine` to manage auction phases.
  - [X] `storage` for type safety with storage operations.
  - [X] `circuit-breaker` for pausable smart contracts.
  - [X] `oracles` to manage market price feed.

### Running tests and building

From the workspace root:

1. Cloning the Repository:
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

## Smart contracts

### litemint-auction-contract

This contract implements timed auctions with support for both open and sealed bid auctions, ascending and descending price mechanisms with linear or compound discount, customizable frequency/rate, _buy now_ option, concurrent and cancellable bids, configurable marketplace commission rate, extendable auctions, easy behaviors plugin via strategy design pattern. For further details, check out the [source and documentation](https://github.com/litemint/litemint-soroban-contracts/tree/master/crates/litemint-auction-contract).

### litemint-royalty-contract

This contract implements multiple royalty payment schemes for non-fungible tokens, including fixed, subscription, and percentage-based models. A key feature is its ability to enforce royalty payments without *isolating* NFTs from the Stellar DEX. Our approach ensures that NFT creators and collectors can freely hold and trade their NFTs from any Stellar DEX compatible service, enjoying an unrestricted sales funnel. For further details, check out the [source and documentation](https://github.com/litemint/litemint-soroban-contracts/tree/master/crates/litemint-royalty-contract).

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
