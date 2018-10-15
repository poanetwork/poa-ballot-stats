use ethabi::Address;

// The `use_contract!` macro triggers several Clippy warnings.
#[cfg_attr(
    feature = "cargo-clippy",
    allow(too_many_arguments, redundant_closure, needless_update)
)]
pub mod v2 {
    use_contract!(key_mgr, "abi/v2/KeysManager.abi.json");
    use_contract!(val_meta, "abi/v2/ValidatorMetadata.abi.json");
    use_contract!(voting, "abi/v2/VotingToChangeKeys.abi.json");
}

// The `use_contract!` macro triggers several Clippy warnings.
#[cfg_attr(
    feature = "cargo-clippy",
    allow(too_many_arguments, redundant_closure, needless_update)
)]
pub mod v1 {
    use_contract!(voting, "abi/v1/VotingToChangeKeys.abi.json");
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct ContractV1V2Addresses {
    pub metadata_address: Address,
    pub keys_manager_address: Address,
    pub voting_to_change_keys_address: Address,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct ContractAddresses {
    pub v1: ContractV1V2Addresses,
    pub v2: ContractV1V2Addresses,
}

impl ContractAddresses {
    pub fn is_keys_manager(&self, addr: &Address) -> bool {
        *addr == self.v1.keys_manager_address || *addr == self.v2.keys_manager_address
    }

    pub fn is_voting(&self, addr: &Address) -> bool {
        *addr == self.v1.voting_to_change_keys_address
            || *addr == self.v2.voting_to_change_keys_address
    }
}
