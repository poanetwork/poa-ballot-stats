extern crate clap;
extern crate colored;
#[macro_use]
extern crate error_chain;
extern crate ethabi;
extern crate web3;

mod cli;
mod error;
mod events;
mod stats;
mod util;
mod validator;

use error::{Error, ErrorKind};
use events::{BallotCreated, ChangeFinalized, Vote};
use stats::Stats;
use std::fs::File;
use util::{ContractExt, TopicFilterExt, Web3LogExt};
use web3::futures::Future;

// TODO: `ethabi_derive` produces unparseable tokens.
// mod voting_to_change_keys {
//     #[derive(EthabiContract)]
//     #[ethabi_contract_options(name = "VotingToChangeKeys", path = "abi/VotingToChangeKeys.json")]
//     struct _Dummy;
// }

/// Finds all logged ballots and returns statistics about how many were missed by each voter.
fn count_votes(
    url: &str,
    verbose: bool,
    voting_abi: &File,
    net_con_abi: &File,
    val_meta_abi: &File,
) -> Result<Stats, Error> {
    let (_eloop, transport) = web3::transports::Http::new(url).unwrap();
    let web3 = web3::Web3::new(transport);
    let voting_contract = ethabi::Contract::load(voting_abi)?;
    let net_con_contract = ethabi::Contract::load(net_con_abi)?;
    let val_meta_contract = ethabi::Contract::load(val_meta_abi)?;

    // TODO: Read contract addresses from chain spec.
    let val_meta_addr = util::parse_address("0xfb9c7fC2a00DfFc53948e3bbeb11F3D4b56C31B8").unwrap();
    let web3_val_meta = web3::contract::Contract::new(web3.eth(), val_meta_addr, val_meta_contract);

    let ballot_event = voting_contract.event("BallotCreated")?;
    let vote_event = voting_contract.event("Vote")?;
    let change_event = net_con_contract.event("ChangeFinalized")?;

    // Find all ballots and voter changes.
    let ballot_or_change_filter = ethabi::TopicFilter {
        topic0: ethabi::Topic::OneOf(vec![ballot_event.signature(), change_event.signature()]),
        ..ethabi::TopicFilter::default()
    }.to_filter_builder()
        .build();
    let ballot_change_logs_filter = web3.eth_filter()
        .create_logs_filter(ballot_or_change_filter)
        .wait()?;

    // FIXME: Find out why we see no `ChangeFinalized` events, and how to obtain the initial voters.
    let mut voters: Vec<ethabi::Address> = Vec::new();
    let mut stats = Stats::default();

    // Iterate over all ballot and voter change events.
    for log in ballot_change_logs_filter.logs().wait()? {
        if let Ok(change_log) = change_event.parse_log(log.clone().into_raw()) {
            // If it is a `ChangeFinalized`, update the current set of voters.
            let change = ChangeFinalized::from_log(&change_log)?;
            if verbose {
                println!("{:?}", change);
            }
            voters = change.new_set;
        } else if let Ok(ballot_log) = ballot_event.parse_log(log.into_raw()) {
            // If it is a `BallotCreated`, find the corresponding votes and update the stats.
            let ballot = BallotCreated::from_log(&ballot_log)?;
            if verbose {
                println!("{:?}", ballot);
            }
            let vote_filter = vote_event
                .create_filter(ballot.vote_topic_filter())?
                .to_filter_builder()
                .build();
            let vote_logs_filter = web3.eth_filter().create_logs_filter(vote_filter).wait()?;
            let mut votes: Vec<Vote> = Vec::new();
            for vote_log in vote_logs_filter.logs().wait()? {
                let vote = Vote::from_log(&vote_event.parse_log(vote_log.into_raw())?)?;
                if !voters.contains(&vote.voter) {
                    eprintln!("Unexpected voter {} for ballot {}", vote.voter, ballot.id);
                    voters.push(vote.voter);
                }
                votes.push(vote);
            }
            stats.add_ballot(&voters, &votes);
        } else {
            return Err(ErrorKind::UnexpectedLogParams.into());
        }
    }

    // Finally, gather the metadata for all voters.
    for voter in voters {
        let mining_key = web3_val_meta.simple_query("getMiningByVotingKey", voter)?;
        let validator = web3_val_meta.simple_query("validators", mining_key)?;
        stats.set_metadata(&voter, mining_key, validator);
    }
    Ok(stats)
}

fn main() {
    let matches = cli::get_matches();
    let url = matches.value_of("url").unwrap_or("http://127.0.0.1:8545");
    let verbose = matches.is_present("verbose");
    let voting_abi = File::open("abi/VotingToChangeKeys.json").expect("read voting abi");
    let net_con_abi = File::open("abi/PoaNetworkConsensus.json").expect("read consensus abi");
    let val_meta_abi = File::open("abi/ValidatorMetadata.json").expect("read val meta abi");
    let stats =
        count_votes(url, verbose, &voting_abi, &net_con_abi, &val_meta_abi).expect("count votes");
    println!("{}", stats);
}
