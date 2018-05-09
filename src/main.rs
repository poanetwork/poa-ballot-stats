extern crate clap;
extern crate colored;
#[macro_use]
extern crate error_chain;
extern crate ethabi;
#[macro_use]
extern crate ethabi_derive;
#[macro_use]
extern crate ethabi_contract;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate web3;

mod cli;
mod error;
mod stats;
mod util;
mod validator;

use error::{Error, ErrorKind};
use stats::Stats;
use std::default::Default;
use std::fs::File;
use util::{ContractExt, HexBytes, HexList, TopicFilterExt, Web3LogExt};
use web3::futures::Future;

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

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct ContractAddresses {
    metadata_address: String,
    keys_manager_address: String,
}

/// Finds all logged ballots and returns statistics about how many were missed by each voter.
fn count_votes(
    url: &str,
    verbose: bool,
    contract_addrs: &ContractAddresses,
) -> Result<Stats, Error> {
    let (_eloop, transport) = web3::transports::Http::new(url).unwrap();
    let web3 = web3::Web3::new(transport);

    let val_meta_abi = File::open("abi/ValidatorMetadata.abi.json").expect("read val meta abi");
    let key_mgr_abi = File::open("abi/KeysManager.abi.json").expect("read key mgr abi");

    let voting_contract = voting::VotingToChangeKeys::default();
    let net_con_contract = net_con::NetworkConsensus::default();
    let val_meta_contract = ethabi::Contract::load(val_meta_abi)?;
    let key_mgr_contract = ethabi::Contract::load(key_mgr_abi)?;

    let val_meta_addr = util::parse_address(&contract_addrs.metadata_address).unwrap();
    let web3_val_meta = web3::contract::Contract::new(web3.eth(), val_meta_addr, val_meta_contract);
    let key_mgr_addr = util::parse_address(&contract_addrs.keys_manager_address).unwrap();
    let web3_key_mgr = web3::contract::Contract::new(web3.eth(), key_mgr_addr, key_mgr_contract);

    let ballot_event = voting_contract.events().ballot_created();
    let vote_event = voting_contract.events().vote();
    let change_event = net_con_contract.events().change_finalized();
    let init_change_event = net_con_contract.events().initiate_change();

    // Find all ballots and voter changes.
    let ballot_or_change_filter =
        (ballot_event.create_filter(ethabi::Topic::Any, ethabi::Topic::Any, ethabi::Topic::Any))
            .or(change_event.create_filter())
            .or(init_change_event.create_filter(ethabi::Topic::Any))
            .to_filter_builder()
            .build();
    let ballot_change_logs_filter = web3.eth_filter()
        .create_logs_filter(ballot_or_change_filter)
        .wait()?;

    // FIXME: Find out why we see no `ChangeFinalized` events, and how to obtain the initial voters.
    let mut voters: Vec<ethabi::Address> = Vec::new();
    let mut stats = Stats::default();
    let mut prev_init_change: Option<net_con::logs::InitiateChange> = None;

    if verbose {
        println!("Collecting events…");
    }
    let mut event_found = false;

    // Iterate over all ballot and voter change events.
    for log in ballot_change_logs_filter.logs().wait()? {
        event_found = true;
        if let Ok(change) = change_event.parse_log(log.clone().into_raw()) {
            // If it is a `ChangeFinalized`, update the current set of voters.
            if verbose {
                println!(
                    "• ChangeFinalized {{ new_set: {} }}",
                    HexList(&change.new_set)
                );
            }
            voters = change.new_set;
        } else if let Ok(init_change) = init_change_event.parse_log(log.clone().into_raw()) {
            // If it is an `InitiateChange`, update the current set of voters.
            if verbose {
                println!(
                    "• InitiateChange {{ parent_hash: {}, new_set: {} }}",
                    HexBytes(&init_change.parent_hash),
                    HexList(&init_change.new_set)
                );
            }
            if let Some(prev) = prev_init_change.take() {
                voters = vec![];
                for mining_key in prev.new_set {
                    let voter = web3_key_mgr.simple_query("getVotingByMining", mining_key)?;
                    if voter != ethabi::Address::zero() {
                        voters.push(voter);
                    }
                }
            }
            prev_init_change = Some(init_change);
        } else if let Ok(ballot) = ballot_event.parse_log(log.into_raw()) {
            // If it is a `BallotCreated`, find the corresponding votes and update the stats.
            if verbose {
                println!("• {:?}", ballot);
            }
            let vote_filter = vote_event
                .create_filter(ballot.id, ethabi::Topic::Any)
                .to_filter_builder()
                .build();
            let vote_logs_filter = web3.eth_filter().create_logs_filter(vote_filter).wait()?;
            let vote_logs = vote_logs_filter.logs().wait()?;
            let votes = vote_logs
                .into_iter()
                .map(|vote_log| {
                    let vote = vote_event.parse_log(vote_log.into_raw())?;
                    if !voters.contains(&vote.voter) {
                        if verbose {
                            eprintln!("  Unexpected voter {}", vote.voter);
                        }
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

    if verbose {
        println!(""); // Add a new line between event log and table.
    }

    // Finally, gather the metadata for all voters.
    for voter in voters {
        let mining_key = match web3_val_meta.simple_query("getMiningByVotingKey", voter) {
            Ok(key) => key,
            Err(err) => {
                eprintln!("Could not fetch mining key for {:?}: {:?}", voter, err);
                continue;
            }
        };
        let validator = web3_val_meta.simple_query("validators", mining_key)?;
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
