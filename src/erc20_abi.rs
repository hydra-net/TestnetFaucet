use web3::ethabi::Function;
use web3::ethabi::Param;
use web3::ethabi::ParamType;
use web3::ethabi::StateMutability;

pub fn transfer() -> Function {
    let input_address = Param {
        name: "_to".to_string(),
        kind: ParamType::Address,
        internal_type: None,
    };

    let input_amount = Param {
        name: "_value".to_string(),
        kind: ParamType::Uint(256),
        internal_type: None,
    };

    let output = Param {
        name: "".to_string(),
        kind: ParamType::Bool,
        internal_type: None,
    };

    let transfer_function = Function {
        name: "transfer".to_string(),
        inputs: vec![input_address, input_amount],
        outputs: vec![output],
        constant: false,
        state_mutability: StateMutability::NonPayable,
    };

    return transfer_function;
}
