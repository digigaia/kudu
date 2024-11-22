use std::str::FromStr;

use color_eyre::eyre::Result;
use serde_json::{json, Value as JsonValue};

use antelope_abi::abidefinition::*;
use antelope_abi::ABI;

// =============================================================================
//
//     Unittests coming from the reference Spring implementation
//     https://github.com/AntelopeIO/spring/blob/main/unittests/abi_tests.cpp
//
//     skipping the following tests because they seem to provide relatively
//     low value:
//      - linkauth_test, unlinkauth_test, updateauth_test, deleteauth_test, ...
//
// =============================================================================

// -----------------------------------------------------------------------------
//     Utility functions & macros
// -----------------------------------------------------------------------------

#[track_caller]
fn verify_round_trip(abi: &ABI, typename: &str, value: &JsonValue) -> Result<()> {
    let encoded = abi.variant_to_binary(typename, value)?;
    let decoded = abi.binary_to_variant(typename, encoded.clone())?;
    let encoded2 = abi.variant_to_binary(typename, &decoded)?;

    // assert_eq!(value, &decoded);
    assert_eq!(encoded, encoded2);
    Ok(())
}

macro_rules! check_error {
    ($t:ident, $error_type:pat) => {
        // let result = $t ;
        assert!($t.is_err(), "expected error, found some result instead");
        match $t.err().unwrap() {
            $error_type => (),
            err => panic!("wrong error type: expected `{}`, got `{}`", stringify!($error_type), err),
        }
    };
    ($t:ident, $error_type:pat, $msg:literal) => {
        assert!($t.is_err(), "expected error, found some result instead");
        let err = $t.err().unwrap();
        match err {
            $error_type => {
                let received = err.to_string();
                if !received.contains($msg) {
                    panic!(r#"expected error with message "{}", got this instead: "{}""#,
                           $msg, received);
                }
            },
            err => panic!("wrong error type: expected `{}`, got `{}`", stringify!($error_type), err),
        }
    };
}


// -----------------------------------------------------------------------------
//     Unittests
// -----------------------------------------------------------------------------

#[test]
fn uint_types() -> Result<()> {
    let currency_abi = r#"
    {
        "version": "eosio::abi/1.0",
        "types": [],
        "structs": [{
            "name": "transfer",
            "base": "",
            "fields": [{
                "name": "amount64",
                "type": "uint64"
            },{
                "name": "amount32",
                "type": "uint32"
            },{
                "name": "amount16",
                "type": "uint16"
            },{
                "name": "amount8",
                "type": "uint8"
            }]
        }],
        "actions": [],
        "tables": [],
        "ricardian_clauses": []
    }
    "#;
    let currency_abi = ABIDefinition::from_str(currency_abi)?;
    let abi = ABI::from_definition(&currency_abi.with_contract_abi()?)?;

    let test_data = json!({
        "amount64": 64,
        "amount32": 32,
        "amount16": 16,
        "amount8": 8,
    });

    verify_round_trip(&abi, "transfer", &test_data)
}


#[test]
fn general() -> Result<()> {
    let my_abi = ABIDefinition::from_str(MY_ABI)?;
    let abi = ABI::from_definition(&my_abi.with_contract_abi()?)?;
    let test_data = JsonValue::from_str(MY_DATA)?;

    verify_round_trip(&abi, "A", &test_data)
}


#[test]
fn abi_cycle() -> Result<()> {
    let typedef_cycle_abi = r#"
    {
        "version": "eosio::abi/1.0",
        "types": [{
            "new_type_name": "A",
            "type": "name"
        },{
            "new_type_name": "name",
            "type": "A"
        }],
        "structs": [],
        "actions": [],
        "tables": [],
        "ricardian_clauses": []
    }
    "#;
    let typedef_cycle_abi = ABIDefinition::from_str(typedef_cycle_abi)?;

    let struct_cycle_abi = r#"
    {
        "version": "eosio::abi/1.0",
        "types": [],
        "structs": [{
            "name": "A",
            "base": "B",
            "fields": []
        },{
            "name": "B",
            "base": "C",
            "fields": []
        },{
            "name": "C",
            "base": "A",
            "fields": []
        }],
        "actions": [],
        "tables": [],
        "ricardian_clauses": []
    }
    "#;
    let struct_cycle_abi = ABIDefinition::from_str(struct_cycle_abi)?;


    let abi = ABI::from_definition(&typedef_cycle_abi);
    println!("{:?}", abi);
    check_error!(abi, ABIError::IntegrityError { .. }, "gloup");

    let abi = ABI::from_definition(&struct_cycle_abi);
    check_error!(abi, ABIError::IntegrityError { .. });

    Ok(())
}

#[test]
fn abi_type_repeat() -> Result<()> {
    let repeat_abi = r#"
    {
        "version": "eosio::abi/1.0",
        "types": [{
            "new_type_name": "actor_name",
            "type": "name"
        },{
            "new_type_name": "actor_name",
            "type": "name"
        }
                 ],
        "structs": [{
            "name": "transfer",
            "base": "",
            "fields": [{
                "name": "from",
                "type": "actor_name"
            },{
                "name": "to",
                "type": "actor_name"
            },{
                "name": "amount",
                "type": "uint64"
            }]
        },{
            "name": "account",
            "base": "",
            "fields": [{
                "name": "account",
                "type": "name"
            },{
                "name": "balance",
                "type": "uint64"
            }]
        }
                   ],
        "actions": [{
            "name": "transfer",
            "type": "transfer"
        }
                   ],
        "tables": [{
            "name": "account",
            "type": "account",
            "index_type": "i64",
            "key_names" : ["account"],
            "key_types" : ["name"]
        }
                  ],
        "ricardian_clauses": []
    }
    "#;
    let repeat_abi = ABIDefinition::from_str(repeat_abi)?;

    let abi = ABI::from_definition(&repeat_abi);
    check_error!(abi, ABIError::IntegrityError { .. });

    Ok(())
}

#[test]
fn abi_struct_repeat() -> Result<()> {
    let repeat_abi = r#"
    {
        "version": "eosio::abi/1.0",
        "types": [{
            "new_type_name": "actor_name",
            "type": "name"
        }
                 ],
        "structs": [{
            "name": "transfer",
            "base": "",
            "fields": [{
                "name": "from",
                "type": "actor_name"
            },{
                "name": "to",
                "type": "actor_name"
            },{
                "name": "amount",
                "type": "uint64"
            }]
        },{
            "name": "transfer",
            "base": "",
            "fields": [{
                "name": "account",
                "type": "name"
            },{
                "name": "balance",
                "type": "uint64"
            }]
        }
                   ],
        "actions": [{
            "name": "transfer",
            "type": "transfer"
        }
                   ],
        "tables": [{
            "name": "account",
            "type": "account",
            "index_type": "i64",
            "key_names" : ["account"],
            "key_types" : ["name"]
        }
                  ],
        "ricardian_clauses": []
    }
    "#;

    let repeat_abi = ABIDefinition::from_str(repeat_abi)?;

    let abi = ABI::from_definition(&repeat_abi);
    check_error!(abi, ABIError::IntegrityError { .. });

    Ok(())
}

// -----------------------------------------------------------------------------
//     Test data
// -----------------------------------------------------------------------------

const MY_ABI: &str = r#"
{
   "version": "eosio::abi/1.0",
   "types": [{
      "new_type_name": "type_name",
      "type": "string"
   },{
      "new_type_name": "field_name",
      "type": "string"
   },{
      "new_type_name": "fields",
      "type": "field_def[]"
   },{
      "new_type_name": "scope_name",
      "type": "name"
   }],
   "structs": [{
      "name": "abi_extension",
      "base": "",
      "fields": [{
         "name": "type",
         "type": "uint16"
      },{
         "name": "data",
         "type": "bytes"
      }]
   },{
      "name": "type_def",
      "base": "",
      "fields": [{
         "name": "new_type_name",
         "type": "type_name"
      },{
         "name": "type",
         "type": "type_name"
      }]
   },{
      "name": "field_def",
      "base": "",
      "fields": [{
         "name": "name",
         "type": "field_name"
      },{
         "name": "type",
         "type": "type_name"
      }]
   },{
      "name": "struct_def",
      "base": "",
      "fields": [{
         "name": "name",
         "type": "type_name"
      },{
         "name": "base",
         "type": "type_name"
      },{
         "name": "fields",
         "type": "field_def[]"
      }]
   },{
      "name": "action_def",
      "base": "",
      "fields": [{
         "name": "name",
         "type": "action_name"
      },{
         "name": "type",
         "type": "type_name"
      },{
         "name": "ricardian_contract",
         "type": "string"
      }]
   },{
      "name": "table_def",
      "base": "",
      "fields": [{
         "name": "name",
         "type": "table_name"
      },{
         "name": "index_type",
         "type": "type_name"
      },{
         "name": "key_names",
         "type": "field_name[]"
      },{
         "name": "key_types",
         "type": "type_name[]"
      },{
         "name": "type",
         "type": "type_name"
      }]
   },{
     "name": "clause_pair",
     "base": "",
     "fields": [{
         "name": "id",
         "type": "string"
     },{
         "name": "body",
         "type": "string"
     }]
   },{
      "name": "abi_def",
      "base": "",
      "fields": [{
         "name": "version",
         "type": "string"
      },{
         "name": "types",
         "type": "type_def[]"
      },{
         "name": "structs",
         "type": "struct_def[]"
      },{
         "name": "actions",
         "type": "action_def[]"
      },{
         "name": "tables",
         "type": "table_def[]"
      },{
         "name": "ricardian_clauses",
         "type": "clause_pair[]"
      },{
         "name": "abi_extensions",
         "type": "abi_extension[]"
      }]
   },{
      "name"  : "A",
      "base"  : "PublicKeyTypes",
      "fields": []
   },{
      "name": "signed_transaction",
      "base": "transaction",
      "fields": [{
         "name": "signatures",
         "type": "signature[]"
      },{
         "name": "context_free_data",
         "type": "bytes[]"
      }]
   },{
      "name": "PublicKeyTypes",
      "base" : "AssetTypes",
      "fields": [{
         "name": "publickey",
         "type": "public_key"
      },{
         "name": "publickey_arr",
         "type": "public_key[]"
      }]
    },{
      "name": "AssetTypes",
      "base" : "NativeTypes",
      "fields": [{
         "name": "asset",
         "type": "asset"
      },{
         "name": "asset_arr",
         "type": "asset[]"
      }]
    },{
      "name": "NativeTypes",
      "fields" : [{
         "name": "string",
         "type": "string"
      },{
         "name": "string_arr",
         "type": "string[]"
      },{
         "name": "block_timestamp_type",
         "type": "block_timestamp_type"
      },{
         "name": "time_point",
         "type": "time_point"
      },{
         "name": "time_point_arr",
         "type": "time_point[]"
      },{
         "name": "time_point_sec",
         "type": "time_point_sec"
      },{
         "name": "time_point_sec_arr",
         "type": "time_point_sec[]"
      },{
         "name": "signature",
         "type": "signature"
      },{
         "name": "signature_arr",
         "type": "signature[]"
      },{
         "name": "checksum256",
         "type": "checksum256"
      },{
         "name": "checksum256_arr",
         "type": "checksum256[]"
      },{
         "name": "fieldname",
         "type": "field_name"
      },{
         "name": "fieldname_arr",
         "type": "field_name[]"
      },{
         "name": "typename",
         "type": "type_name"
      },{
         "name": "typename_arr",
         "type": "type_name[]"
      },{
         "name": "uint8",
         "type": "uint8"
      },{
         "name": "uint8_arr",
         "type": "uint8[]"
      },{
         "name": "uint16",
         "type": "uint16"
      },{
         "name": "uint16_arr",
         "type": "uint16[]"
      },{
         "name": "uint32",
         "type": "uint32"
      },{
         "name": "uint32_arr",
         "type": "uint32[]"
      },{
         "name": "uint64",
         "type": "uint64"
      },{
         "name": "uint64_arr",
         "type": "uint64[]"
      },{
         "name": "uint128",
         "type": "uint128"
      },{
         "name": "uint128_arr",
         "type": "uint128[]"
      },{
         "name": "int8",
         "type": "int8"
      },{
         "name": "int8_arr",
         "type": "int8[]"
      },{
         "name": "int16",
         "type": "int16"
      },{
         "name": "int16_arr",
         "type": "int16[]"
      },{
         "name": "int32",
         "type": "int32"
      },{
         "name": "int32_arr",
         "type": "int32[]"
      },{
         "name": "int64",
         "type": "int64"
      },{
         "name": "int64_arr",
         "type": "int64[]"
      },{
         "name": "int128",
         "type": "int128"
      },{
         "name": "int128_arr",
         "type": "int128[]"
      },{
         "name": "name",
         "type": "name"
      },{
         "name": "name_arr",
         "type": "name[]"
      },{
         "name": "field",
         "type": "field_def"
      },{
         "name": "field_arr",
         "type": "field_def[]"
      },{
         "name": "struct",
         "type": "struct_def"
      },{
         "name": "struct_arr",
         "type": "struct_def[]"
      },{
         "name": "fields",
         "type": "fields"
      },{
         "name": "fields_arr",
         "type": "fields[]"
      },{
         "name": "accountname",
         "type": "account_name"
      },{
         "name": "accountname_arr",
         "type": "account_name[]"
      },{
         "name": "permname",
         "type": "permission_name"
      },{
         "name": "permname_arr",
         "type": "permission_name[]"
      },{
         "name": "actionname",
         "type": "action_name"
      },{
         "name": "actionname_arr",
         "type": "action_name[]"
      },{
         "name": "scopename",
         "type": "scope_name"
      },{
         "name": "scopename_arr",
         "type": "scope_name[]"
      },{
         "name": "permlvl",
         "type": "permission_level"
      },{
         "name": "permlvl_arr",
         "type": "permission_level[]"
      },{
         "name": "action",
         "type": "action"
      },{
         "name": "action_arr",
         "type": "action[]"
      },{
         "name": "permlvlwgt",
         "type": "permission_level_weight"
      },{
         "name": "permlvlwgt_arr",
         "type": "permission_level_weight[]"
      },{
         "name": "transaction",
         "type": "transaction"
      },{
         "name": "transaction_arr",
         "type": "transaction[]"
      },{
         "name": "strx",
         "type": "signed_transaction"
      },{
         "name": "strx_arr",
         "type": "signed_transaction[]"
      },{
         "name": "keyweight",
         "type": "key_weight"
      },{
         "name": "keyweight_arr",
         "type": "key_weight[]"
      },{
         "name": "authority",
         "type": "authority"
      },{
         "name": "authority_arr",
         "type": "authority[]"
      },{
         "name": "typedef",
         "type": "type_def"
      },{
         "name": "typedef_arr",
         "type": "type_def[]"
      },{
         "name": "actiondef",
         "type": "action_def"
      },{
         "name": "actiondef_arr",
         "type": "action_def[]"
      },{
         "name": "tabledef",
         "type": "table_def"
      },{
         "name": "tabledef_arr",
         "type": "table_def[]"
      },{
         "name": "abidef",
         "type": "abi_def"
      },{
         "name": "abidef_arr",
         "type": "abi_def[]"
      }]
    }
  ],
  "actions": [],
  "tables": [],
  "ricardian_clauses": [{"id":"clause A","body":"clause body A"},
              {"id":"clause B","body":"clause body B"}],
  "abi_extensions": []
}
"#;

const MY_DATA: &str = r#"
    {
      "publickey"     :  "EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV",
      "publickey_arr" :  ["EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV","EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV","EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV"],
      "asset"         : "100.0000 SYS",
      "asset_arr"     : ["100.0000 SYS","100.0000 SYS"],

      "string"            : "ola ke ase",
      "string_arr"        : ["ola ke ase","ola ke desi"],
      "block_timestamp_type" : "2021-12-20T15:00",
      "time_point"        : "2021-12-20T15:30",
      "time_point_arr"    : ["2021-12-20T15:30","2021-12-20T15:31"],
      "time_point_sec"    : "2021-12-20T15:30:21",
      "time_point_sec_arr": ["2021-12-20T15:30:21","2021-12-20T15:31:21"],
      "signature"         : "SIG_K1_Jzdpi5RCzHLGsQbpGhndXBzcFs8vT5LHAtWLMxPzBdwRHSmJkcCdVu6oqPUQn1hbGUdErHvxtdSTS1YA73BThQFwV1v4G5",
      "signature_arr"     : ["SIG_K1_Jzdpi5RCzHLGsQbpGhndXBzcFs8vT5LHAtWLMxPzBdwRHSmJkcCdVu6oqPUQn1hbGUdErHvxtdSTS1YA73BThQFwV1v4G5","SIG_K1_Jzdpi5RCzHLGsQbpGhndXBzcFs8vT5LHAtWLMxPzBdwRHSmJkcCdVu6oqPUQn1hbGUdErHvxtdSTS1YA73BThQFwV1v4G5"],
      "checksum256"       : "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
      "checksum256_arr"      : ["ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad","ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"],
      "fieldname"         : "name1",
      "fieldname_arr"     : ["name1","name2"],
      "typename"          : "name3",
      "typename_arr"      : ["name4","name5"],
      "bytes"             : "010203",
      "bytes_arr"         : ["010203","","040506"],
      "uint8"             : 8,
      "uint8_arr"         : [8,9],
      "uint16"            : 16,
      "uint16_arr"        : [16,17],
      "uint32"            : 32,
      "uint32_arr"        : [32,33],
      "uint64"            : 64,
      "uint64_arr"        : [64,65],
      "uint128"           : 128,
      "uint128_arr"       : ["0x00000000000000000000000000000080",129],
      "int8"              : 108,
      "int8_arr"          : [108,109],
      "int16"             : 116,
      "int16_arr"         : [116,117],
      "int32"             : 132,
      "int32_arr"         : [132,133],
      "int64"             : 164,
      "int64_arr"         : [164,165],
      "int128"            : -128,
      "int128_arr"        : ["0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF80",-129],
      "name"              : "xname1",
      "name_arr"          : ["xname2","xname3"],
      "field"             : {"name":"name1", "type":"type1"},
      "field_arr"         : [{"name":"name1", "type":"type1"}, {"name":"name2", "type":"type2"}],
      "struct"            : {"name":"struct1", "base":"base1", "fields": [{"name":"name1", "type":"type1"}, {"name":"name2", "type":"type2"}]},
      "struct_arr"        : [{"name":"struct1", "base":"base1", "fields": [{"name":"name1", "type":"type1"}, {"name":"name2", "type":"type2"}]},{"name":"struct1", "base":"base1", "fields": [{"name":"name1", "type":"type1"}, {"name":"name2", "type":"type2"}]}],
      "fields"            : [{"name":"name1", "type":"type1"}, {"name":"name2", "type":"type2"}],
      "fields_arr"        : [[{"name":"name1", "type":"type1"}, {"name":"name2", "type":"type2"}],[{"name":"name3", "type":"type3"}, {"name":"name4", "type":"type4"}]],
      "accountname"       : "acc1",
      "accountname_arr"   : ["acc1","acc2"],
      "permname"          : "pername",
      "permname_arr"      : ["pername1","pername2"],
      "actionname"        : "actionname",
      "actionname_arr"    : ["actionname1","actionname2"],
      "scopename"         : "acc1",
      "scopename_arr"     : ["acc1","acc2"],
      "permlvl"           : {"actor":"acc1","permission":"permname1"},
      "permlvl_arr"       : [{"actor":"acc1","permission":"permname1"},{"actor":"acc2","permission":"permname2"}],
      "action"            : {"account":"acc1", "name":"actionname1", "authorization":[{"actor":"acc1","permission":"permname1"}], "data":"445566"},
      "action_arr"        : [{"account":"acc1", "name":"actionname1", "authorization":[{"actor":"acc1","permission":"permname1"}], "data":"445566"},{"account":"acc2", "name":"actionname2", "authorization":[{"actor":"acc2","permission":"permname2"}], "data":""}],
      "permlvlwgt"        : {"permission":{"actor":"acc1","permission":"permname1"},"weight":"1"},
      "permlvlwgt_arr"    : [{"permission":{"actor":"acc1","permission":"permname1"},"weight":"1"},{"permission":{"actor":"acc2","permission":"permname2"},"weight":"2"}],
      "transaction"       : {
        "ref_block_num":"1",
        "ref_block_prefix":"2",
        "expiration":"2021-12-20T15:30",
        "context_free_actions":[{"account":"contextfree1", "name":"cfactionname1", "authorization":[{"actor":"cfacc1","permission":"cfpermname1"}], "data":"778899"}],
        "actions":[{"account":"accountname1", "name":"actionname1", "authorization":[{"actor":"acc1","permission":"permname1"}], "data":"445566"}],
        "max_net_usage_words":15,
        "max_cpu_usage_ms":43,
        "delay_sec":0,
        "transaction_extensions": []
      },
      "transaction_arr": [{
        "ref_block_num":"1",
        "ref_block_prefix":"2",
        "expiration":"2021-12-20T15:30",
        "context_free_actions":[{"account":"contextfree1", "name":"cfactionname1", "authorization":[{"actor":"cfacc1","permission":"cfpermname1"}], "data":"778899"}],
        "actions":[{"account":"acc1", "name":"actionname1", "authorization":[{"actor":"acc1","permission":"permname1"}], "data":"445566"}],
        "max_net_usage_words":15,
        "max_cpu_usage_ms":43,
        "delay_sec":0,
        "transaction_extensions": []
      },{
        "ref_block_num":"2",
        "ref_block_prefix":"3",
        "expiration":"2021-12-20T15:40",
        "context_free_actions":[{"account":"contextfree1", "name":"cfactionname1", "authorization":[{"actor":"cfacc1","permission":"cfpermname1"}], "data":"778899"}],
        "actions":[{"account":"acc2", "name":"actionname2", "authorization":[{"actor":"acc2","permission":"permname2"}], "data":""}],
        "max_net_usage_words":21,
        "max_cpu_usage_ms":87,
        "delay_sec":0,
        "transaction_extensions": []
      }],
      "strx": {
        "ref_block_num":"1",
        "ref_block_prefix":"2",
        "expiration":"2021-12-20T15:30",
        "region": "1",
        "signatures" : ["SIG_K1_Jzdpi5RCzHLGsQbpGhndXBzcFs8vT5LHAtWLMxPzBdwRHSmJkcCdVu6oqPUQn1hbGUdErHvxtdSTS1YA73BThQFwV1v4G5"],
        "context_free_data" : ["abcdef","0123456789","ABCDEF0123456789abcdef"],
        "context_free_actions":[{"account":"contextfree1", "name":"cfactionname1", "authorization":[{"actor":"cfacc1","permission":"cfpermname1"}], "data":"778899"}],
        "actions":[{"account":"accountname1", "name":"actionname1", "authorization":[{"actor":"acc1","permission":"permname1"}], "data":"445566"}],
        "max_net_usage_words":15,
        "max_cpu_usage_ms":43,
        "delay_sec":0,
        "transaction_extensions": []
      },
      "strx_arr": [{
        "ref_block_num":"1",
        "ref_block_prefix":"2",
        "expiration":"2021-12-20T15:30",
        "region": "1",
        "signatures" : ["SIG_K1_Jzdpi5RCzHLGsQbpGhndXBzcFs8vT5LHAtWLMxPzBdwRHSmJkcCdVu6oqPUQn1hbGUdErHvxtdSTS1YA73BThQFwV1v4G5"],
        "context_free_data" : ["abcdef","0123456789","ABCDEF0123456789abcdef"],
        "context_free_actions":[{"account":"contextfree1", "name":"cfactionname1", "authorization":[{"actor":"cfacc1","permission":"cfpermname1"}], "data":"778899"}],
        "actions":[{"account":"acc1", "name":"actionname1", "authorization":[{"actor":"acc1","permission":"permname1"}], "data":"445566"}],
        "max_net_usage_words":15,
        "max_cpu_usage_ms":43,
        "delay_sec":0,
        "transaction_extensions": []
      },{
        "ref_block_num":"2",
        "ref_block_prefix":"3",
        "expiration":"2021-12-20T15:40",
        "region": "1",
        "signatures" : ["SIG_K1_Jzdpi5RCzHLGsQbpGhndXBzcFs8vT5LHAtWLMxPzBdwRHSmJkcCdVu6oqPUQn1hbGUdErHvxtdSTS1YA73BThQFwV1v4G5"],
        "context_free_data" : ["abcdef","0123456789","ABCDEF0123456789abcdef"],
        "context_free_actions":[{"account":"contextfree2", "name":"cfactionname2", "authorization":[{"actor":"cfacc2","permission":"cfpermname2"}], "data":"667788"}],
        "actions":[{"account":"acc2", "name":"actionname2", "authorization":[{"actor":"acc2","permission":"permname2"}], "data":""}],
        "max_net_usage_words":15,
        "max_cpu_usage_ms":43,
        "delay_sec":0,
        "transaction_extensions": []
      }],
      "keyweight": {"key":"EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV", "weight":"100"},
      "keyweight_arr": [{"key":"EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV", "weight":"100"},{"key":"EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV", "weight":"200"}],
      "authority": {
         "threshold":"10",
         "keys":[{"key":"EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV", "weight":100},{"key":"EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV", "weight":200}],
         "accounts":[{"permission":{"actor":"acc1","permission":"permname1"},"weight":"1"},{"permission":{"actor":"acc2","permission":"permname2"},"weight":"2"}],
         "waits":[]
       },
      "authority_arr": [{
         "threshold":"10",
         "keys":[{"key":"EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV", "weight":"100"},{"key":"EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV", "weight":"200"}],
         "accounts":[{"permission":{"actor":"acc1","permission":"permname1"},"weight":"1"},{"permission":{"actor":"acc2","permission":"permname2"},"weight":"2"}],
         "waits":[]
       },{
         "threshold":"10",
         "keys":[{"key":"EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV", "weight":"100"},{"key":"EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV", "weight":"200"}],
         "accounts":[{"permission":{"actor":"acc1","permission":"permname1"},"weight":"1"},{"permission":{"actor":"acc2","permission":"permname2"},"weight":"2"}],
         "waits":[]
       }],
      "typedef" : {"new_type_name":"new", "type":"old"},
      "typedef_arr": [{"new_type_name":"new", "type":"old"},{"new_type_name":"new", "type":"old"}],
      "actiondef"       : {"name":"actionname1", "type":"type1", "ricardian_contract":"ricardian1"},
      "actiondef_arr"   : [{"name":"actionname1", "type":"type1","ricardian_contract":"ricardian1"},{"name":"actionname2", "type":"type2","ricardian_contract":"ricardian2"}],
      "tabledef": {"name":"table1","index_type":"indextype1","key_names":["keyname1"],"key_types":["typename1"],"type":"type1"},
      "tabledef_arr": [
         {"name":"table1","index_type":"indextype1","key_names":["keyname1"],"key_types":["typename1"],"type":"type1"},
         {"name":"table2","index_type":"indextype2","key_names":["keyname2"],"key_types":["typename2"],"type":"type2"}
      ],
      "abidef":{
        "version": "eosio::abi/1.0",
        "types" : [{"new_type_name":"new", "type":"old"}],
        "structs" : [{"name":"struct1", "base":"base1", "fields": [{"name":"name1", "type": "type1"}, {"name":"name2", "type": "type2"}] }],
        "actions" : [{"name":"action1","type":"type1", "ricardian_contract":""}],
        "tables" : [{"name":"table1","index_type":"indextype1","key_names":["keyname1"],"key_types":["typename1"],"type":"type1"}],
        "ricardian_clauses": [],
        "abi_extensions": []
      },
      "abidef_arr": [{
        "version": "eosio::abi/1.0",
        "types" : [{"new_type_name":"new", "type":"old"}],
        "structs" : [{"name":"struct1", "base":"base1", "fields": [{"name":"name1", "type": "type1"}, {"name":"name2", "type": "type2"}] }],
        "actions" : [{"name":"action1","type":"type1", "ricardian_contract":""}],
        "tables" : [{"name":"table1","index_type":"indextype1","key_names":["keyname1"],"key_types":["typename1"],"type":"type1"}],
        "ricardian_clauses": [],
        "abi_extensions": []
      },{
        "version": "eosio::abi/1.0",
        "types" : [{"new_type_name":"new", "type":"old"}],
        "structs" : [{"name":"struct1", "base":"base1", "fields": [{"name":"name1", "type": "type1"}, {"name":"name2", "type": "type2"}] }],
        "actions" : [{"name":"action1","type":"type1", "ricardian_contract": ""}],
        "tables" : [{"name":"table1","index_type":"indextype1","key_names":["keyname1"],"key_types":["typename1"],"type":"type1"}],
        "ricardian_clauses": [],
        "abi_extensions": []
      }]
    }
"#;
