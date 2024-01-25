pub static TEST_ABI: &str = r#"{
    "version": "eosio::abi/1.1",
    "structs": [
        {
            "name": "s1",
            "fields": [
                {
                    "name": "x1",
                    "type": "int8"
                }
            ]
        },
        {
            "name": "s2",
            "fields": [
                {
                    "name": "y1",
                    "type": "int8$"
                },
                {
                    "name": "y2",
                    "type": "int8$"
                }
            ]
        },
        {
            "name": "s3",
            "fields": [
                {
                    "name": "z1",
                    "type": "int8$"
                },
                {
                    "name": "z2",
                    "type": "v1$"
                },
                {
                    "name": "z3",
                    "type": "s2$"
                }
            ]
        },
        {
            "name": "s4",
            "fields": [
                {
                    "name": "a1",
                    "type": "int8?$"
                },
                {
                    "name": "b1",
                    "type": "int8[]$"
                }
            ]
        },
        {
            "name": "s5",
            "fields": [
                {
                    "name": "x1",
                    "type": "int8"
                },
                {
                    "name": "x2",
                    "type": "int8"
                },
                {
                    "name": "x3",
                    "type": "s6"
                }
            ]
        },
        {
            "name": "s6",
            "fields": [
                {
                    "name": "c1",
                    "type": "int8"
                },
                {
                    "name": "c2",
                    "type": "s5[]"
                },
                {
                    "name": "c3",
                    "type": "int8"
                }
            ]
        }
    ],
    "variants": [
        {
            "name": "v1",
            "types": ["int8","s1","s2"]
        }
    ]
}"#;
