use colored::{Color, Colorize};
use ethabi::Address;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use validator::Validator;
use voting;

/// The count of ballots and cast votes, as well as metadata for a particular voter.
#[derive(Clone, Default)]
struct VoterStats {
    /// The number of ballots where this voter had a right to vote.
    ballots: usize,
    /// The number of votes cast by this voter.
    voted: usize,
    /// The validator metadata.
    validator: Option<Validator>,
    /// The mining key.
    mining_key: Option<Address>,
}

/// A map of vote counts, by voting key.
#[derive(Clone, Default)]
pub struct Stats {
    voter_stats: HashMap<Address, VoterStats>,
}

impl Stats {
    /// Adds a ballot: `voters` are the voting keys of everyone who was allowed to cast a vote, and
    /// `votes` are the ones that were actually cast.
    pub fn add_ballot(&mut self, voters: &[Address], votes: &[voting::logs::Vote]) {
        for voter in voters {
            let mut vs = self
                .voter_stats
                .entry(voter.clone())
                .or_insert_with(VoterStats::default);
            vs.ballots += 1;
            if votes.iter().any(|vote| vote.voter == *voter) {
                vs.voted += 1;
            }
        }
    }

    /// Inserts metadata about a voter: the mining key and the `Validator` information.
    pub fn set_metadata(
        &mut self,
        voter: &Address,
        mining_key: Address,
        validator: Validator,
    ) -> bool {
        match self.voter_stats.get_mut(voter) {
            None => false,
            Some(vs) => {
                vs.validator = Some(validator);
                vs.mining_key = Some(mining_key);
                true
            }
        }
    }
}

impl Display for Stats {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut lines: Vec<DisplayLine> = self
            .voter_stats
            .iter()
            .map(|(addr, s)| DisplayLine {
                votes_per_thousand: s.voted * 1000 / s.ballots,
                voted: s.voted,
                ballots: s.ballots,
                voting_address: *addr,
                mining_key: match s.mining_key {
                    None => "".to_string(),
                    Some(ref key) => format!("{}", key),
                },
                name: match s.validator {
                    None => "".to_string(),
                    Some(ref v) => format!("{} {}", v.first_name, v.last_name),
                },
            })
            .collect();
        lines.sort();
        let header = "        Missed  Voting key   Mining key   Name".bold();
        writeln!(f, "{}", header)?;
        for line in lines {
            line.fmt(f)?;
        }
        Ok(())
    }
}

/// A line in the output, corresponding to a particular voter.
#[derive(Ord, PartialOrd, Eq, PartialEq)]
struct DisplayLine {
    votes_per_thousand: usize,
    voted: usize,
    ballots: usize,
    voting_address: Address,
    mining_key: String,
    name: String,
}

impl Display for DisplayLine {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let text = format!(
            "{:>7}, {:4.1}%  {}  {}  {}",
            format!("{}/{}", self.ballots - self.voted, self.ballots),
            100.0 - (self.votes_per_thousand as f32) / 10.0,
            self.voting_address,
            self.mining_key,
            self.name
        );
        let c = if self.votes_per_thousand <= 500 {
            Color::BrightRed
        } else if self.votes_per_thousand <= 750 {
            Color::BrightYellow
        } else if self.votes_per_thousand < 1000 {
            Color::White
        } else {
            Color::BrightGreen
        };
        writeln!(f, "{}", text.color(c))
    }
}
