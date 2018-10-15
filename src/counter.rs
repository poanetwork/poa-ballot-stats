use colored::{Color, Colorize};
use contracts::v1::voting::events::{ballot_created as ballot_created_v1, vote as vote_v1};
use contracts::v2::key_mgr::events::voting_key_changed;
use contracts::v2::key_mgr::functions::get_mining_key_by_voting;
use contracts::v2::val_meta::functions::validators as validators_fn;
use contracts::v2::voting::events::{ballot_created, vote};
use contracts::ContractAddresses;
use error::{Error, ErrorKind};
use ethabi::{Address, Bytes, FunctionOutputDecoder, Uint};
use stats::Stats;
use std::collections::BTreeSet;
use std::default::Default;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use util::{self, HexList, IntoBallot, TopicFilterExt, Web3LogExt};
use web3;
use web3::futures::Future;

/// The maximum age in seconds of the latest block.
const MAX_BLOCK_AGE: u64 = 60 * 60;

const ERR_BLOCK_NUM: &str = "event is missing block number";
const ERR_EPOCH: &str = "current timestamp is earlier than the Unix epoch";
const ERR_BLOCK: &str = "failed to retrieve block";

/// A vote counter, to read ballot statistics from the blockchain.
pub struct Counter {
    verbose: bool,
    start_time: SystemTime,
    start_block: u64,
    addrs: ContractAddresses,
    web3: web3::Web3<web3::transports::Http>,
    _eloop: web3::transports::EventLoopHandle,
}

impl Counter {
    /// Creates a new vote counter.
    pub fn new(url: &str, addrs: ContractAddresses) -> Counter {
        let (_eloop, transport) = web3::transports::Http::new(url).unwrap();
        let web3 = web3::Web3::new(transport);

        Counter {
            verbose: false,
            start_time: UNIX_EPOCH,
            start_block: 0,
            addrs,
            web3,
            _eloop,
        }
    }

    /// Enables verbose mode.
    pub fn set_verbose(&mut self) {
        self.verbose = true;
    }

    /// Sets the first block to be taken into account, by creation time.
    pub fn set_start_time(&mut self, start_time: SystemTime) {
        self.start_time = start_time;
    }

    /// Sets the first block to be taken into account, by number.
    pub fn set_start_block(&mut self, start_block: u64) {
        self.start_block = start_block;
    }

    /// Finds all logged ballots and returns statistics about how many were missed by each voter.
    pub fn count_votes(&mut self) -> Result<Stats, Error> {
        self.check_synced();

        // Calls `println!` if `verbose` is `true`.
        macro_rules! vprintln { ($($arg:tt)*) => { if self.verbose { println!($($arg)*); } } }

        // Find all ballots and voter changes. We don't filter by contract address, so we can make
        // a single pass. Contract addresses are checked inside the loop.
        let ballot_or_change_filter = ballot_created::filter(None, None, None)
            .or(ballot_created_v1::filter(None, None, None))
            .or(voting_key_changed::filter(None));

        let mut voters: BTreeSet<Address> = BTreeSet::new();
        let mut stats = Stats::default();

        vprintln!("Collecting events…");
        let mut event_found = false;

        // Iterate over all ballot and voter change events.
        for log in ballot_or_change_filter.logs(&self.web3)? {
            let block_num = log.block_number.expect(ERR_BLOCK_NUM).into();
            if let Ok(change) = voting_key_changed::parse_log(log.clone().into_raw()) {
                if !self.addrs.is_keys_manager(&log.address) {
                    continue; // Event from another contract instance.
                }
                event_found = true;
                // If it is a `VotingKeyChanged`, update the current set of voters.
                vprintln!("• {} {:?}", format!("#{}", block_num).bold(), change);
                match change.action.as_str() {
                    "added" => {
                        voters.insert(change.key);
                    }
                    "removed" => {
                        voters.remove(&change.key);
                    }
                    _ => vprintln!("  Unexpected key change action."),
                }
            } else if let Ok(ballot) =
                ballot_created::parse_log(log.clone().into_raw()).or_else(|_| {
                    ballot_created_v1::parse_log(log.clone().into_raw()).map(IntoBallot::into)
                }) {
                if !self.addrs.is_voting(&log.address) {
                    continue; // Event from another contract instance.
                }
                event_found = true;
                if block_num < self.start_block || self.is_block_too_old(block_num) {
                    let num = format!("#{}", block_num);
                    vprintln!("• {} Ballot too old; skipping: {:?}", num.bold(), ballot);
                    continue;
                }
                // If it is a `BallotCreated`, find the corresponding votes and update the stats.
                vprintln!("• {} {:?}", format!("#{}", block_num).bold(), ballot);
                let voted = self.voters_for_ballot(ballot.id)?;
                if self.verbose {
                    self.print_ballot_details(&voters, &voted);
                }
                voters.extend(voted.iter().cloned());
                stats.add_ballot(&voters, &voted);
            } else {
                return Err(ErrorKind::UnexpectedLogParams.into());
            }
        }

        if !event_found {
            return Err(ErrorKind::NoEventsFound.into());
        }

        vprintln!(""); // Add a new line between event log and table.

        // Finally, gather the metadata for all voters.
        for voter in voters {
            let mining_key = match self.call_key_mgr(get_mining_key_by_voting::call(voter)) {
                Err(err) => {
                    eprintln!("Failed to find mining key for voter {}: {:?}", voter, err);
                    continue;
                }
                Ok(key) => key,
            };
            if mining_key.is_zero() {
                eprintln!("Mining key for voter {} is zero. Skipping.", voter);
                continue;
            }
            let validator = self.call_val_meta(validators_fn::call(mining_key))?.into();
            stats.set_metadata(&voter, mining_key, validator);
        }
        Ok(stats)
    }

    fn print_ballot_details(&self, voters: &BTreeSet<Address>, voted: &[Address]) {
        let mut unexpected = BTreeSet::new();
        let mut expected = BTreeSet::new();
        for voter in voted {
            if voters.contains(voter) {
                expected.insert(*voter);
            } else {
                unexpected.insert(*voter);
            }
        }
        let missed_filter = |voter: &&Address| !voted.contains(voter);
        let missed: BTreeSet<_> = voters.iter().filter(missed_filter).collect();
        if !missed.is_empty() {
            println!("  Missed: {}", HexList(&missed, Color::Red));
        }
        if !expected.is_empty() {
            println!("  Voted: {}", HexList(voted, Color::Green));
        }
        if !unexpected.is_empty() {
            println!("  Unexpected: {}", HexList(&unexpected, Color::Yellow));
        }
    }

    /// Calls a function of the `ValidatorMetadata` contract and returns the decoded result.
    fn call_val_meta<D>(&self, fn_call: (Bytes, D)) -> Result<D::Output, web3::contract::Error>
    where
        D: FunctionOutputDecoder,
    {
        util::raw_call(self.addrs.v2.metadata_address, &self.web3.eth(), fn_call)
    }

    /// Calls a function of the `KeysManager` contract and returns the decoded result.
    fn call_key_mgr<D>(&self, fn_call: (Bytes, D)) -> Result<D::Output, web3::contract::Error>
    where
        D: FunctionOutputDecoder,
    {
        util::raw_call(
            self.addrs.v2.keys_manager_address,
            &self.web3.eth(),
            fn_call,
        )
    }

    fn voters_for_ballot(&self, id: Uint) -> Result<Vec<Address>, Error> {
        let vote_filter = vote::filter(id, None).or(vote_v1::filter(id, None));
        let is_voting = |log: &web3::types::Log| self.addrs.is_voting(&log.address);
        vote_filter
            .logs(&self.web3)?
            .into_iter()
            .filter(is_voting)
            .map(|vote_log| {
                vote::parse_log(vote_log.clone().into_raw())
                    .map(|vote| vote.voter)
                    .or_else(|_| vote_v1::parse_log(vote_log.into_raw()).map(|vote| vote.voter))
                    .map_err(Error::from)
            }).collect()
    }

    /// Returns `true` if the block with the given number is older than `start_time`.
    fn is_block_too_old(&self, block_num: u64) -> bool {
        self.is_block_older_than(
            web3::types::BlockNumber::Number(block_num),
            &self.start_time,
        )
    }

    /// Shows a warning if the node's latest block is outdated.
    fn check_synced(&self) {
        let min_time = SystemTime::now() - Duration::from_secs(MAX_BLOCK_AGE);
        if self.is_block_older_than(web3::types::BlockNumber::Latest, &min_time) {
            eprintln!("WARNING: The node is not fully synchronized. Stats may be inaccurate.");
        }
    }

    /// Returns `true` if the block with the given number was created before the given time.
    fn is_block_older_than(&self, number: web3::types::BlockNumber, time: &SystemTime) -> bool {
        let id = web3::types::BlockId::Number(number);
        let block_result = self.web3.eth().block(id).wait();
        let block = block_result.expect(ERR_BLOCK).expect(ERR_BLOCK);
        let seconds = time.duration_since(UNIX_EPOCH).expect(ERR_EPOCH).as_secs();
        block.timestamp < seconds.into()
    }
}
