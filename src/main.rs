extern crate clap;
extern crate colored;
#[macro_use]
extern crate error_chain;
extern crate ethabi;
#[macro_use(EthabiContract)]
extern crate ethabi_derive;
#[macro_use(use_contract)]
extern crate ethabi_contract;
extern crate serde;
#[macro_use(Deserialize)]
extern crate serde_derive;
extern crate serde_json;
extern crate web3;

mod cli;
mod error;
mod stats;
mod util;
mod validator;

use error::{Error, ErrorKind};
use ethabi::Address;
use stats::Stats;
use std::default::Default;
use std::fs::File;
use std::time::{SystemTime, UNIX_EPOCH};
use util::{HexBytes, HexList, TopicFilterExt, Web3LogExt};
use web3::futures::Future;

/// The maximum age in seconds of the latest block.
const MAX_BLOCK_AGE: u64 = 60 * 60;

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

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct ContractAddresses {
    metadata_address: String,
    keys_manager_address: String,
}

/// Shows a warning if the node's latest block is outdated.
fn check_synced<T: web3::Transport>(web3: &web3::Web3<T>) {
    let id = web3::types::BlockId::Number(web3::types::BlockNumber::Latest);
    let block = web3.eth().block(id).wait().expect("get latest block");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Current timestamp is earlier than the Unix epoch!");
    if block.timestamp < (now.as_secs() - MAX_BLOCK_AGE).into() {
        eprintln!("WARNING: The node is not fully synchronized. Stats may be inaccurate.");
    }
}

/// Finds all logged ballots and returns statistics about how many were missed by each voter.
fn count_votes(
    url: &str,
    verbose: bool,
    contract_addrs: &ContractAddresses,
) -> Result<Stats, Error> {
    // Calls `println!` if `verbose` is `true`.
    macro_rules! vprintln { ($($arg:tt)*) => { if verbose { println!($($arg)*); } } }

    let (_eloop, transport) = web3::transports::Http::new(url).unwrap();
    let web3 = web3::Web3::new(transport);

    check_synced(&web3);

    let voting_contract = voting::VotingToChangeKeys::default();
    let net_con_contract = net_con::NetworkConsensus::default();
    let val_meta_contract = val_meta::ValidatorMetadata::default();
    let key_mgr_contract = key_mgr::KeysManager::default();

    let val_meta_addr =
        util::parse_address(&contract_addrs.metadata_address).expect("parse contract address");
    let key_mgr_addr =
        util::parse_address(&contract_addrs.keys_manager_address).expect("parse contract address");

    let ballot_event = voting_contract.events().ballot_created();
    let vote_event = voting_contract.events().vote();
    let change_event = net_con_contract.events().change_finalized();
    let init_change_event = net_con_contract.events().initiate_change();

    // Find all ballots and voter changes.
    let ballot_or_change_filter = (ballot_event.create_filter(None, None, None))
        .or(change_event.create_filter())
        .or(init_change_event.create_filter(None));

    // FIXME: Find out why we see no `ChangeFinalized` events, and how to obtain the initial voters.
    let mut voters: Vec<Address> = Vec::new();
    let mut stats = Stats::default();
    let mut prev_init_change: Option<net_con::logs::InitiateChange> = None;

    vprintln!("Collecting events…");
    let mut event_found = false;

    // Iterate over all ballot and voter change events.
    for log in ballot_or_change_filter.logs(&web3)? {
        event_found = true;
        if let Ok(change) = change_event.parse_log(log.clone().into_raw()) {
            // If it is a `ChangeFinalized`, update the current set of voters.
            vprintln!(
                "• ChangeFinalized {{ new_set: {} }}",
                HexList(&change.new_set)
            );
            voters = change.new_set;
        } else if let Ok(init_change) = init_change_event.parse_log(log.clone().into_raw()) {
            // If it is an `InitiateChange`, update the current set of voters.
            vprintln!(
                "• InitiateChange {{ parent_hash: {}, new_set: {} }}",
                HexBytes(&init_change.parent_hash),
                HexList(&init_change.new_set)
            );
            if let Some(prev) = prev_init_change.take() {
                let raw_call = util::raw_call(key_mgr_addr, web3.eth());
                let get_voting_by_mining_fn = key_mgr_contract.functions().get_voting_by_mining();
                voters = vec![];
                for mining_key in prev.new_set {
                    let voter = get_voting_by_mining_fn.call(mining_key, &*raw_call)?;
                    if voter != Address::zero() {
                        voters.push(voter);
                    }
                }
            }
            prev_init_change = Some(init_change);
        } else if let Ok(ballot) = ballot_event.parse_log(log.into_raw()) {
            // If it is a `BallotCreated`, find the corresponding votes and update the stats.
            vprintln!("• {:?}", ballot);
            let votes = vote_event
                .create_filter(ballot.id, None)
                .logs(&web3)?
                .into_iter()
                .map(|vote_log| {
                    let vote = vote_event.parse_log(vote_log.into_raw())?;
                    if !voters.contains(&vote.voter) {
                        vprintln!("  Unexpected voter {}", vote.voter);
                        voters.push(vote.voter);
                    }
                    Ok(vote)
                })
                .collect::<Result<Vec<_>, Error>>()?;
            stats.add_ballot(&voters, &votes);
        } else {
            return Err(ErrorKind::UnexpectedLogParams.into());
        }
    }

    if !event_found {
        return Err(ErrorKind::NoEventsFound.into());
    }

    vprintln!(""); // Add a new line between event log and table.

    // Finally, gather the metadata for all voters.
    let raw_call = util::raw_call(val_meta_addr, web3.eth());
    let get_mining_by_voting_key_fn = val_meta_contract.functions().get_mining_by_voting_key();
    let validators_fn = val_meta_contract.functions().validators();
    for voter in voters {
        let mining_key = match get_mining_by_voting_key_fn.call(voter, &*raw_call) {
            Err(err) => {
                eprintln!("Failed to find mining key for voter {}: {:?}", voter, err);
                continue;
            }
            Ok(key) => key,
        };
        let validator = validators_fn.call(mining_key, &*raw_call)?.into();
        stats.set_metadata(&voter, mining_key, validator);
    }
    Ok(stats)
}

fn main() {
    let matches = cli::get_matches();
    let url = matches.value_of("url").unwrap_or("http://127.0.0.1:8545");
    let verbose = matches.is_present("verbose");
    let contract_file = matches
        .value_of("contracts")
        .unwrap_or("contracts/core.json");
    let file = File::open(contract_file).expect("open contracts file");
    let contract_addrs = serde_json::from_reader(file).expect("parse contracts file");
    let stats = count_votes(url, verbose, &contract_addrs).expect("count votes");
    println!("{}", stats);
}
