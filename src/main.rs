extern crate clap;
extern crate colored;
#[macro_use]
extern crate error_chain;
extern crate ethabi;
#[macro_use(EthabiContract)]
extern crate ethabi_derive;
#[macro_use(use_contract)]
extern crate ethabi_contract;
extern crate parse_duration;
extern crate serde;
#[macro_use(Deserialize)]
extern crate serde_derive;
extern crate serde_json;
extern crate web3;

mod cli;
mod counter;
mod error;
mod stats;
mod util;
mod validator;

use std::fs::File;
use std::time::{SystemTime, UNIX_EPOCH};

// The `use_contract!` macro triggers several Clippy warnings.
#[cfg_attr(feature = "cargo-clippy", allow(too_many_arguments, redundant_closure, needless_update))]
mod contracts {
    use_contract!(
        net_con,
        "NetworkConsensus",
        "abi/PoaNetworkConsensus.abi.json"
    );
    use_contract!(
        voting,
        "VotingToChangeKeys",
        "abi/VotingToChangeKeys.abi.json"
    );
    use_contract!(
        val_meta,
        "ValidatorMetadata",
        "abi/ValidatorMetadata.abi.json"
    );
    use_contract!(key_mgr, "KeysManager", "abi/KeysManager.abi.json");
}

use contracts::*;

fn main() {
    let matches = cli::get_matches();
    let url = matches.value_of("url").unwrap_or("http://127.0.0.1:8545");
    let verbose = matches.is_present("verbose");
    let contract_file = matches
        .value_of("contracts")
        .unwrap_or("contracts/core.json");
    let file = File::open(contract_file).expect("open contracts file");
    let contract_addrs = serde_json::from_reader(file).expect("parse contracts file");
    let start = matches
        .value_of("period")
        .map(|period| {
            let duration = parse_duration::parse(period)
                .expect("period must be in the format '5 days', '2 months', etc.");
            SystemTime::now() - duration
        })
        .unwrap_or(UNIX_EPOCH);
    let start_block = matches.value_of("block").map_or(0, |block| {
        block
            .parse()
            .expect("block number must be a non-negative integer")
    });
    let stats = counter::count_votes(url, verbose, &contract_addrs, start, start_block)
        .expect("count votes");
    println!("{}", stats);
}
