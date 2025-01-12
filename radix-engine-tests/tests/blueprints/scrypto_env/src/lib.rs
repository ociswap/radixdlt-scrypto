use scrypto::api::node_modules::royalty::COMPONENT_ROYALTY_SETTER_ROLE;
use scrypto::prelude::*;

#[blueprint]
mod scrypto_env_test {
    struct ScryptoEnvTest {}

    impl ScryptoEnvTest {
        pub fn create_node_with_invalid_blueprint() {
            ScryptoVmV1Api::object_new(
                "invalid_blueprint",
                btreemap![0u8 => FieldValue::new(&ScryptoEnvTest {})],
            );
        }

        pub fn create_and_open_mut_substate_twice(heap: bool) {
            let obj = Self {}.instantiate();
            if heap {
                obj.open_mut_substate_twice();
                obj.prepare_to_globalize(OwnerRole::None).globalize();
            } else {
                let globalized = obj.prepare_to_globalize(OwnerRole::None).globalize();
                globalized.open_mut_substate_twice();
            }
        }

        pub fn open_mut_substate_twice(&mut self) {
            ScryptoVmV1Api::actor_open_field(ACTOR_STATE_SELF, 0u8, LockFlags::MUTABLE);

            ScryptoVmV1Api::actor_open_field(ACTOR_STATE_SELF, 0u8, LockFlags::MUTABLE);
        }

        pub fn bech32_encode_address(address: ComponentAddress) -> String {
            Runtime::bech32_encode_address(address)
        }
    }
}

#[blueprint]
mod local_auth_zone {
    struct LocalAuthZoneTest {}

    impl LocalAuthZoneTest {
        pub fn pop_empty_auth_zone() -> Option<Proof> {
            LocalAuthZone::pop()
        }
    }
}

#[blueprint]
mod max_sbor_depth {
    use sbor::basic_well_known_types::ANY_TYPE;
    use sbor::*;

    struct MaxSborDepthTest {
        kv_store: Own,
    }

    impl MaxSborDepthTest {
        pub fn write_kv_store_entry_with_depth(buffer: Vec<u8>) {
            // Create KeyValueStore<Any, Any>
            let schema = VersionedScryptoSchema::V1(SchemaV1 {
                type_kinds: vec![],
                type_metadata: vec![],
                type_validations: vec![],
            });
            let schema_hash = schema.generate_schema_hash();
            let kv_store = ScryptoVmV1Api::kv_store_new(KeyValueStoreGenericArgs {
                additional_schema: Some(schema),
                key_type: GenericSubstitution::Local(TypeIdentifier(
                    schema_hash,
                    LocalTypeIndex::from(ANY_TYPE),
                )),
                value_type: GenericSubstitution::Local(TypeIdentifier(
                    schema_hash,
                    LocalTypeIndex::from(ANY_TYPE),
                )),
                allow_ownership: false,
            });

            // Open entry
            let handle = ScryptoVmV1Api::kv_store_open_entry(
                &kv_store,
                &scrypto_encode("key").unwrap(),
                LockFlags::MUTABLE,
            );

            // Write entry
            ScryptoVmV1Api::kv_entry_write(handle, buffer);

            // Clean up
            Self {
                kv_store: Own(kv_store),
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .globalize();
        }
    }
}
