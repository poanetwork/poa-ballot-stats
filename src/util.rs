use ethabi;
use std::{fmt, u8};
use web3;
use web3::futures::Future;

// TODO: Evaluate whether any of these would make sense to include in `web3`.

/// Converts the bytes to a string, interpreting them as null-terminated UTF-8.
pub fn bytes_to_string(bytes: &[u8]) -> String {
    let zero = bytes
        .iter()
        .position(|b| *b == 0)
        .unwrap_or_else(|| bytes.len());
    String::from_utf8_lossy(&bytes[..zero]).to_string()
}

/// Parses the string as a 40-digit hexadecimal number, and returns the corresponding `Address`.
pub fn parse_address(mut s: &str) -> Option<ethabi::Address> {
    let mut bytes = [0u8; 20];
    if &s[..2] == "0x" {
        s = &s[2..];
    }
    for i in 0..20 {
        match u8::from_str_radix(&s[(2 * i)..(2 * i + 2)], 16) {
            Ok(b) => bytes[i] = b,
            Err(_) => return None,
        }
    }
    Some(ethabi::Address::from_slice(&bytes))
}

pub trait ContractExt {
    fn simple_query<P, R>(&self, func: &str, params: P) -> Result<R, web3::contract::Error>
    where
        R: web3::contract::tokens::Detokenize,
        P: web3::contract::tokens::Tokenize;
}

impl ContractExt for web3::contract::Contract<web3::transports::Http> {
    /// Calls a constant function with the latest block and default parameters.
    fn simple_query<P, R>(&self, func: &str, params: P) -> Result<R, web3::contract::Error>
    where
        R: web3::contract::tokens::Detokenize,
        P: web3::contract::tokens::Tokenize,
    {
        self.query(
            func,
            params,
            None,
            web3::contract::Options::default(),
            web3::types::BlockNumber::Latest,
        ).wait()
    }
}

trait TopicExt<T> {
    /// Returns the union of the two topics.
    fn or(self, other: Self) -> Self;

    /// Converts this topic into an `Option<Vec<T>>`, where `Any` corresponds to `None`,
    /// `This` to a vector with one element, and `OneOf` to any vector.
    fn to_opt_vec(self) -> Option<Vec<T>>;
}

impl<T: Ord> TopicExt<T> for ethabi::Topic<T> {
    fn or(self, other: Self) -> Self {
        match (self.to_opt_vec(), other.to_opt_vec()) {
            (Some(mut v0), Some(v1)) => {
                for e in v1 {
                    if !v0.contains(&e) {
                        v0.push(e);
                    }
                }
                if v0.len() == 1 {
                    ethabi::Topic::This(v0.into_iter().next().expect("has a single element; qed"))
                } else {
                    ethabi::Topic::OneOf(v0)
                }
            }
            (_, _) => ethabi::Topic::Any,
        }
    }

    fn to_opt_vec(self) -> Option<Vec<T>> {
        match self {
            ethabi::Topic::Any => None,
            ethabi::Topic::OneOf(v) => Some(v),
            ethabi::Topic::This(t) => Some(vec![t]),
        }
    }
}

pub trait TopicFilterExt {
    /// Returns a `web3::types::FilterBuilder` with these topics, starting from the first block.
    fn to_filter_builder(self) -> web3::types::FilterBuilder;

    /// Returns the "disjunction" of the two filters, i.e. it filters for everything that matches
    /// at least one of the two in every topic.
    fn or(self, other: ethabi::TopicFilter) -> ethabi::TopicFilter;
}

impl TopicFilterExt for ethabi::TopicFilter {
    fn to_filter_builder(self) -> web3::types::FilterBuilder {
        web3::types::FilterBuilder::default()
            .topics(
                self.topic0.to_opt_vec(),
                self.topic1.to_opt_vec(),
                self.topic2.to_opt_vec(),
                self.topic3.to_opt_vec(),
            )
            .from_block(web3::types::BlockNumber::Earliest)
            .to_block(web3::types::BlockNumber::Latest)
    }

    fn or(self, other: ethabi::TopicFilter) -> ethabi::TopicFilter {
        ethabi::TopicFilter {
            topic0: self.topic0.or(other.topic0),
            topic1: self.topic1.or(other.topic1),
            topic2: self.topic2.or(other.topic2),
            topic3: self.topic3.or(other.topic3),
        }
    }
}

pub trait Web3LogExt {
    fn into_raw(self) -> ethabi::RawLog;
}

impl Web3LogExt for web3::types::Log {
    fn into_raw(self) -> ethabi::RawLog {
        (self.topics, self.data.0).into()
    }
}

pub trait LogExt {
    /// Returns the `i`-th parameter, if it has the given name, otherwise `None`.
    fn param(&self, i: usize, name: &str) -> Option<&ethabi::Token>;

    /// Returns the `i`-th parameter, if it is an `Address` and has the given name, otherwise
    /// `None`.
    fn address_param(&self, i: usize, name: &str) -> Option<&ethabi::Address>;

    /// Returns the `i`-th parameter, if it is a `Uint` and has the given name, otherwise `None`.
    fn uint_param(&self, i: usize, name: &str) -> Option<&ethabi::Uint>;
}

impl LogExt for ethabi::Log {
    fn param(&self, i: usize, name: &str) -> Option<&ethabi::Token> {
        self.params.get(i).and_then(|param| {
            if param.name == name {
                Some(&param.value)
            } else {
                None
            }
        })
    }

    fn address_param(&self, i: usize, name: &str) -> Option<&ethabi::Address> {
        match self.param(i, name) {
            Some(&ethabi::Token::Address(ref address)) => Some(address),
            _ => None,
        }
    }

    fn uint_param(&self, i: usize, name: &str) -> Option<&ethabi::Uint> {
        match self.param(i, name) {
            Some(&ethabi::Token::Uint(ref i)) => Some(i),
            _ => None,
        }
    }
}

/// Wrapper for a byte array, whose `Display` implementation outputs shortened hexadecimal strings.
pub struct HexBytes<'a>(pub &'a [u8]);

impl<'a> fmt::Display for HexBytes<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x")?;
        for i in &self.0[..2] {
            write!(f, "{:02x}", i)?;
        }
        write!(f, "…")?;
        for i in &self.0[(self.0.len() - 2)..] {
            write!(f, "{:02x}", i)?;
        }
        Ok(())
    }
}

/// Wrapper for a list of byte arrays, whose `Display` implementation outputs shortened hexadecimal
/// strings.
pub struct HexList<'a, T: 'a>(pub &'a [T]);

impl<'a, T: 'a> fmt::Display for HexList<'a, T>
where
    T: AsRef<[u8]>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for (i, item) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", HexBytes(item.as_ref()))?;
        }
        write!(f, "]")
    }
}
