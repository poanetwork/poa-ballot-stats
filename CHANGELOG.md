# Changelog
All notable changes to the project will be documented in this file.


## Unreleased

- Migrate to Rust 2018. At least version 1.31.0 required.
- Show new validators in the list, even if there was no ballot since they were added.


## [0.4.0] - 2018-10-15

- Support new contracts (hard fork 2).


## [0.3.0] - 2018-05-29
### Added
- --b flag to specify a block integer. This option displays records starting with a block number.
- Check of contract address to ensure event handling comes from the correct contracts instance
- Release binaries to allow for program to run without Cargo installed
- Additional code commenting and error messaging

### Changed
- Moved counter code to separate module at counter.rs and refactored counter code
- -v option extended; displays full lists of participating and abstaining voters for each ballot

### Fixed
- Switched to `VotingKeyChanged` event to track the current validator set and confirm a mining key is finalized rather than `InitializeChange` event, which was not always finalized.


## [0.2.1] - 2018-05-24

### Fixed
- Web3 compilation issue.

## [0.2.0] - 2018-05-21
### Added
-  -p flag to allow user to display limited time periods in which ballots are counted

### Changed
- Display changed to show current validators only. Validators who have been removed are no longer shown.

### Fixed
- Fixed server filter registration to allow for use with load-balanced servers and compatibility with https://core.poa.network 


## [0.1.0] - 2018-05-19
### Added
- Initial implementation
- GPL3 License
- Enabled Travis CI
- ABI and contract address files updated from [poa-chain-spec](https://github.com/poanetwork/poa-chain-spec)
- Use Ethabi Contract
- Reference to [RP9 specifications](https://github.com/poanetwork/RFC/issues/9)
- Enabled build scripts
- Checks for node sychonization / error messages if nodes are not synced
- Parsing for validator addresses
- Updated dependencies including Rust and Clippy


[0.4.0]: https://github.com/poanetwork/poa-ballot-stats/releases/tag/0.4.0
[0.3.0]: https://github.com/poanetwork/poa-ballot-stats/releases/tag/0.3.0
[0.2.1]: https://github.com/poanetwork/poa-ballot-stats/releases/tag/0.2.1
[0.2.0]: https://github.com/poanetwork/poa-ballot-stats/releases/tag/0.2.0
[0.1.0]: https://github.com/poanetwork/poa-ballot-stats/releases/tag/0.1.0
