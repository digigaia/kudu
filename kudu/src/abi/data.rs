
// -----------------------------------------------------------------------------
//     Core data
// -----------------------------------------------------------------------------

pub static ABI_SCHEMA: &str = include_str!("data/abi_definition.json");

pub static CONTRACT_ABI: &str = include_str!("data/contract_abi.json");

// -----------------------------------------------------------------------------
//     Tests data
// -----------------------------------------------------------------------------

// FIXME: find why using #[cfg(test)] makes the tests not compile

#[cfg(test)]
pub static ABI_EXAMPLE: &str = include_str!("data/abi_example.json");

pub static TEST_ABI: &str = include_str!("data/test_abi.json");

pub static TRANSACTION_ABI: &str = include_str!("data/transaction_abi.json");

pub static PACKED_TRANSACTION_ABI: &str = include_str!("data/packed_transaction_abi.json");

pub static KV_TABLES_ABI: &str = include_str!("data/kv_tables_abi.json");

pub static STATE_HISTORY_PLUGIN_ABI: &str = include_str!("data/ship_abi.json");

pub static TOKEN_HEX_ABI: &str = concat!(
    "0e656f73696f3a3a6162692f312e30010c6163636f756e745f6e616d65046e61",
    "6d6505087472616e7366657200040466726f6d0c6163636f756e745f6e616d65",
    "02746f0c6163636f756e745f6e616d65087175616e7469747905617373657404",
    "6d656d6f06737472696e67066372656174650002066973737565720c6163636f",
    "756e745f6e616d650e6d6178696d756d5f737570706c79056173736574056973",
    "737565000302746f0c6163636f756e745f6e616d65087175616e746974790561",
    "73736574046d656d6f06737472696e67076163636f756e7400010762616c616e",
    "63650561737365740e63757272656e63795f7374617473000306737570706c79",
    "0561737365740a6d61785f737570706c79056173736574066973737565720c61",
    "63636f756e745f6e616d6503000000572d3ccdcd087472616e73666572000000",
    "000000a531760569737375650000000000a86cd4450663726561746500020000",
    "00384f4d113203693634010863757272656e6379010675696e74363407616363",
    "6f756e740000000000904dc603693634010863757272656e6379010675696e74",
    "36340e63757272656e63795f7374617473000000");
