use ethabi;
use std::u8;
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

pub trait TopicFilterExt {
    /// Returns a `web3::types::FilterBuilder` with these topics, starting from the first block.
    fn to_filter_builder(self) -> web3::types::FilterBuilder;
}

impl TopicFilterExt for ethabi::TopicFilter {
    fn to_filter_builder(self) -> web3::types::FilterBuilder {
        web3::types::FilterBuilder::default()
            .topics(
                to_topic(self.topic0),
                to_topic(self.topic1),
                to_topic(self.topic2),
                to_topic(self.topic3),
            )
            .from_block(web3::types::BlockNumber::Earliest)
            .to_block(web3::types::BlockNumber::Latest)
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

/// Converts an `ethabi::Topic<T>` into an `Option<Vec<T>>`, where `Any` corresponds to `None`,
/// `This` to a vector with one element, and `OneOf` to any vector.
fn to_topic<T>(topic: ethabi::Topic<T>) -> Option<Vec<T>> {
    match topic {
        ethabi::Topic::Any => None,
        ethabi::Topic::OneOf(v) => Some(v),
        ethabi::Topic::This(t) => Some(vec![t]),
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
