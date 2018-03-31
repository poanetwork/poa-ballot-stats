use error::{ErrorKind, Result};
use ethabi::{Address, Log, RawTopicFilter, Token, Topic, Uint};
use util::LogExt;

/// An event that is logged when the current set of validators has changed.
#[derive(Debug)]
pub struct ChangeFinalized {
    /// The new set of validators.
    pub new_set: Vec<Address>,
}

impl ChangeFinalized {
    /// Parses the log and returns a `ChangeFinalized`, if the log corresponded to such an event.
    pub fn from_log(log: &Log) -> Result<ChangeFinalized> {
        log.param(0, "newSet")
            .cloned()
            .and_then(Token::to_array)
            .map(|tokens| ChangeFinalized {
                new_set: tokens.into_iter().filter_map(Token::to_address).collect(),
            })
            .ok_or_else(|| ErrorKind::UnexpectedLogParams.into())
    }
}

/// An event that is logged when a new ballot is started.
#[derive(Debug)]
pub struct BallotCreated {
    /// The ballot ID.
    pub id: Uint,
    /// The ballot type.
    ballot_type: Uint,
    /// The creator's voting key.
    creator: Address,
}

impl BallotCreated {
    /// Parses the log and returns a `BallotCreated`, if the log corresponded to such an event.
    pub fn from_log(log: &Log) -> Result<BallotCreated> {
        match (
            log.uint_param(0, "id"),
            log.uint_param(1, "ballotType"),
            log.address_param(2, "creator"),
        ) {
            (Some(&id), Some(&ballot_type), Some(&creator)) => Ok(BallotCreated {
                id,
                ballot_type,
                creator,
            }),
            _ => Err(ErrorKind::UnexpectedLogParams.into()),
        }
    }

    /// Returns a topic filter to find the votes corresponding to this ballot.
    pub fn vote_topic_filter(&self) -> RawTopicFilter {
        RawTopicFilter {
            topic0: Topic::This(Token::Uint(self.id)),
            ..RawTopicFilter::default()
        }
    }
}

/// An event that is logged whenever someone casts a vote in a ballot.
#[derive(Debug)]
pub struct Vote {
    /// The ballot ID.
    id: Uint,
    /// The decision this vote is for.
    decision: Uint,
    /// The voter's voting key.
    pub voter: Address,
    /// The timestamp of this vote.
    time: Uint,
}

impl Vote {
    /// Parses the log and returns a `Vote`, if the log corresponded to such an event.
    pub fn from_log(log: &Log) -> Result<Vote> {
        match (
            log.uint_param(0, "id"),
            log.uint_param(1, "decision"),
            log.address_param(2, "voter"),
            log.uint_param(3, "time"),
        ) {
            (Some(&id), Some(&decision), Some(&voter), Some(&time)) => Ok(Vote {
                id,
                decision,
                voter,
                time,
            }),
            _ => Err(ErrorKind::UnexpectedLogParams.into()),
        }
    }
}
