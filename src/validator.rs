use ethabi::Token;
use util;
use web3::contract::{tokens, Error, ErrorKind};

/// Validator metadata.
#[derive(Clone, Debug)]
pub struct Validator {
    pub first_name: String,
    pub last_name: String,
    // bytes32 licenseId,
    // string fullAddress,
    // bytes32 state,
    // uint256 zipcode,
    // uint256 expirationDate,
    // uint256 createdDate,
    // uint256 updatedDate,
    // uint256 minThreshold,
}

impl tokens::Detokenize for Validator {
    /// Returns a `Validator` if the token's types match the fields.
    fn from_tokens(tokens: Vec<Token>) -> Result<Validator, Error> {
        match (tokens.get(0), tokens.get(1)) {
            (Some(&Token::FixedBytes(ref first)), Some(&Token::FixedBytes(ref last))) => {
                Ok(Validator {
                    first_name: util::bytes_to_string(first),
                    last_name: util::bytes_to_string(last),
                })
            }
            _ => Err(ErrorKind::InvalidOutputType("Validator".to_string()).into()),
        }
    }
}
