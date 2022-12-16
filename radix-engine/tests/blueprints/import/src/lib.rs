use scrypto::prelude::*;

import! {
     r#"
        {
            "package_address": "056967d3d49213394892980af59be76e9b3e7cc4cb78237460d0c7",
            "blueprint_name": "Simple",
            "abi": {
                "structure": {
                    "type": "Struct",
                    "name": "Simple",
                    "fields": {
                        "type": "Named",
                        "named": []
                    }
                },
                "fns": [
                    {
                        "ident": "new",
                        "input": {
                            "type": "Struct",
                            "name": "",
                            "fields": {
                                "type": "Named",
                                "named": []
                            }
                        },
                        "output": {
                            "type": "ComponentAddress"
                        },
                        "export_name": "Simple_new_main"
                    },
                    {
                        "ident": "free_token",
                        "mutability": "Mutable",
                        "input": {
                            "type": "Struct",
                            "name": "",
                            "fields": {
                                "type": "Named",
                                "named": []
                            }
                        },
                        "output": {
                            "type": "Bucket"
                        },
                        "export_name": "Simple_free_token_main"
                    }
                ]
            }
        }
    "#
}

blueprint! {
    struct Import {}

    impl Import {}
}
