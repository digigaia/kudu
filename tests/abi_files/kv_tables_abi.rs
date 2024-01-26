pub static TEST_KV_TABLES_ABI: &str = r#"{
    "version": "eosio::abi/1.2",
    "types": [],
    "structs": [
        {
            "name": "get",
            "base": "",
            "fields": []
        },
        {
            "name": "iteration",
            "base": "",
            "fields": []
        },
        {
            "name": "my_struct",
            "base": "",
            "fields": [
                {
                    "name": "primary",
                    "type": "name"
                },
                {
                    "name": "foo",
                    "type": "string"
                },
                {
                    "name": "bar",
                    "type": "uint64"
                },
                {
                    "name": "fullname",
                    "type": "string"
                },
                {
                    "name": "age",
                    "type": "uint32"
                }
            ]
        },
        {
            "name": "nonunique",
            "base": "",
            "fields": []
        },
        {
            "name": "setup",
            "base": "",
            "fields": []
        },
        {
            "name": "tuple_string_uint32",
            "base": "",
            "fields": [
                {
                    "name": "field_0",
                    "type": "string"
                },
                {
                    "name": "field_1",
                    "type": "uint32"
                }
            ]
        },
        {
            "name": "update",
            "base": "",
            "fields": []
        },
        {
            "name": "updateerr1",
            "base": "",
            "fields": []
        },
        {
            "name": "updateerr2",
            "base": "",
            "fields": []
        }
    ],
    "actions": [
        {
            "name": "get",
            "type": "get",
            "ricardian_contract": ""
        },
        {
            "name": "iteration",
            "type": "iteration",
            "ricardian_contract": ""
        },
        {
            "name": "nonunique",
            "type": "nonunique",
            "ricardian_contract": ""
        },
        {
            "name": "setup",
            "type": "setup",
            "ricardian_contract": ""
        },
        {
            "name": "update",
            "type": "update",
            "ricardian_contract": ""
        },
        {
            "name": "updateerr1",
            "type": "updateerr1",
            "ricardian_contract": ""
        },
        {
            "name": "updateerr2",
            "type": "updateerr2",
            "ricardian_contract": ""
        }
    ],
    "tables": []
}"#;
