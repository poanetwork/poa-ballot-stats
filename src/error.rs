use ethabi;
use web3;

error_chain! {
    foreign_links {
        Ethabi(ethabi::Error);
        Web3(web3::Error);
        Contract(web3::contract::Error);
    }

    errors {
        UnexpectedLogParams {
            description("Unexpected parameter types in log"),
        }
    }
}
