use colored::Colorize;
use contracts::{key_mgr, net_con, val_meta, voting};
use error::{Error, ErrorKind};
use ethabi::Address;
use stats::Stats;
use std::default::Default;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use util::{self, HexBytes, HexList, TopicFilterExt, Web3LogExt};
use web3;
use web3::futures::Future;

/// The maximum age in seconds of the latest block.
const MAX_BLOCK_AGE: u64 = 60 * 60;

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct ContractAddresses {
    metadata_address: String,
    keys_manager_address: String,
}

/// A vote counter, to read ballot statistics from the blockchain.
pub struct Counter {
    verbose: bool,
    start_time: SystemTime,
    start_block: u64,
    val_meta_addr: Address,
    key_mgr_addr: Address,
    web3: web3::Web3<web3::transports::Http>,
    _eloop: web3::transports::EventLoopHandle,
}

impl Counter {
    /// Creates a new vote counter.
    pub fn new(url: &str, contract_addrs: &ContractAddresses) -> Counter {
        let (_eloop, transport) = web3::transports::Http::new(url).unwrap();
        let web3 = web3::Web3::new(transport);

        let val_meta_addr =
            util::parse_address(&contract_addrs.metadata_address).expect("parse contract address");
        let key_mgr_addr = util::parse_address(&contract_addrs.keys_manager_address)
            .expect("parse contract address");

        Counter {
            verbose: false,
            start_time: UNIX_EPOCH,
            start_block: 0,
            val_meta_addr,
            key_mgr_addr,
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

        let voting_contract = voting::VotingToChangeKeys::default();
        let net_con_contract = net_con::NetworkConsensus::default();
        let val_meta_contract = val_meta::ValidatorMetadata::default();
        let key_mgr_contract = key_mgr::KeysManager::default();

        let ballot_event = voting_contract.events().ballot_created();
        let vote_event = voting_contract.events().vote();
        let change_event = net_con_contract.events().change_finalized();
        let init_change_event = net_con_contract.events().initiate_change();

        // Find all ballots and voter changes.
        let ballot_or_change_filter = (ballot_event.create_filter(None, None, None))
            .or(change_event.create_filter())
            .or(init_change_event.create_filter(None));

        let mut voters: Vec<Address> = Vec::new();
        let mut stats = Stats::default();
        let mut prev_init_change: Option<net_con::logs::InitiateChange> = None;

        vprintln!("Collecting events…");
        let mut event_found = false;

        // Iterate over all ballot and voter change events.
        for log in ballot_or_change_filter.logs(&self.web3)? {
            event_found = true;
            let block_num = log
                .block_number
                .expect("event is missing block number")
                .into();
            if let Ok(change) = change_event.parse_log(log.clone().into_raw()) {
                // If it is a `ChangeFinalized`, update the current set of voters.
                vprintln!(
                    "• {} ChangeFinalized {{ new_set: {} }}",
                    format!("#{}", block_num).bold(),
                    HexList(&change.new_set)
                );
                voters = change.new_set;
            } else if let Ok(init_change) = init_change_event.parse_log(log.clone().into_raw()) {
                // If it is an `InitiateChange`, update the current set of voters.
                vprintln!(
                    "• {} InitiateChange {{ parent_hash: {}, new_set: {} }}",
                    format!("#{}", block_num).bold(),
                    HexBytes(&init_change.parent_hash),
                    HexList(&init_change.new_set)
                );
                if let Some(prev) = prev_init_change.take() {
                    let raw_call = util::raw_call(self.key_mgr_addr, self.web3.eth());
                    let get_voting_by_mining_fn =
                        key_mgr_contract.functions().get_voting_by_mining();
                    voters = vec![];
                    for mining_key in prev.new_set {
                        let voter = get_voting_by_mining_fn.call(mining_key, &*raw_call)?;
                        if voter != Address::zero() {
                            voters.push(voter);
                        }
                    }
                }
                prev_init_change = Some(init_change);
            } else if let Ok(ballot) = ballot_event.parse_log(log.clone().into_raw()) {
                let block_number = web3::types::BlockNumber::Number(block_num);
                if block_num < self.start_block
                    || self.is_block_older_than(block_number, &self.start_time)
                {
                    vprintln!(
                        "• {} Ballot event too old; skipping: {:?}",
                        format!("#{}", block_num).bold(),
                        ballot
                    );
                    continue;
                }
                // If it is a `BallotCreated`, find the corresponding votes and update the stats.
                vprintln!("• {} {:?}", format!("#{}", block_num).bold(), ballot);
                let votes = vote_event
                    .create_filter(ballot.id, None)
                    .logs(&self.web3)?
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
        let raw_call = util::raw_call(self.val_meta_addr, self.web3.eth());
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
        let block = self.web3.eth().block(id).wait().expect("get block");
        let seconds = time
            .duration_since(UNIX_EPOCH)
            .expect("Current timestamp is earlier than the Unix epoch!")
            .as_secs();
        block.timestamp < seconds.into()
    }
}
