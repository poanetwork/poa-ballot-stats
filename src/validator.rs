use ethabi;

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

type ValidatorTuple = (
    ethabi::Hash,
    ethabi::Hash,
    ethabi::Hash,
    String,
    ethabi::Hash,
    ethabi::Hash,
    ethabi::Uint,
    ethabi::Uint,
    ethabi::Uint,
    ethabi::Uint,
);

impl From<ValidatorTuple> for Validator {
    fn from((first_name_h, last_name_h, ..): ValidatorTuple) -> Validator {
        Validator {
            first_name: String::from_utf8_lossy(&*first_name_h)
                .to_owned()
                .to_string(),
            last_name: String::from_utf8_lossy(&*last_name_h)
                .to_owned()
                .to_string(),
        }
    }
}
