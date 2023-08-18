use crate::system::system::KeyValueEntrySubstate;
use crate::types::*;
use crate::{errors::*, event_schema, roles_template};
use native_sdk::runtime::Runtime;
use radix_engine_interface::api::field_api::LockFlags;
use radix_engine_interface::api::node_modules::metadata::*;
use radix_engine_interface::api::{ClientApi, CollectionIndex, FieldValue, GenericArgs, KVEntry, OBJECT_HANDLE_SELF};
use radix_engine_interface::blueprints::package::{
    AuthConfig, BlueprintDefinitionInit, BlueprintType, FunctionAuth, MethodAuthTemplate,
    PackageDefinition,
};
use radix_engine_interface::schema::{
    BlueprintCollectionSchema, BlueprintFunctionsSchemaInit, BlueprintSchemaInit,
    BlueprintStateSchemaInit, FunctionSchemaInit, TypeRef,
};

use super::{RemoveMetadataEvent, SetMetadataEvent};

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub enum MetadataPanicError {
    KeyStringExceedsMaxLength { max: usize, actual: usize },
    ValueSborExceedsMaxLength { max: usize, actual: usize },
    ValueDecodeError(DecodeError),
}

pub const METADATA_COLLECTION: CollectionIndex = 0u8;

pub type MetadataEntrySubstate = KeyValueEntrySubstate<MetadataValue>;

pub struct MetadataNativePackage;

impl MetadataNativePackage {
    pub fn definition() -> PackageDefinition {
        let mut aggregator = TypeAggregator::<ScryptoCustomTypeKind>::new();

        let mut collections = Vec::new();
        collections.push(BlueprintCollectionSchema::KeyValueStore(
            BlueprintKeyValueSchema {
                key: TypeRef::Static(aggregator.add_child_type_and_descendents::<String>()),
                value: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<MetadataValue>(),
                ),
                allow_ownership: false,
            },
        ));

        let mut functions = BTreeMap::new();
        functions.insert(
            METADATA_CREATE_IDENT.to_string(),
            FunctionSchemaInit {
                receiver: None,
                input: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<MetadataCreateInput>(),
                ),
                output: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<MetadataCreateOutput>(),
                ),
                export: METADATA_CREATE_IDENT.to_string(),
            },
        );
        functions.insert(
            METADATA_CREATE_WITH_DATA_IDENT.to_string(),
            FunctionSchemaInit {
                receiver: None,
                input: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<MetadataCreateWithDataInput>(),
                ),
                output: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<MetadataCreateWithDataOutput>(),
                ),
                export: METADATA_CREATE_WITH_DATA_IDENT.to_string(),
            },
        );
        functions.insert(
            METADATA_SET_IDENT.to_string(),
            FunctionSchemaInit {
                receiver: Some(ReceiverInfo::normal_ref_mut()),
                input: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<MetadataSetInput>(),
                ),
                output: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<MetadataSetOutput>(),
                ),
                export: METADATA_SET_IDENT.to_string(),
            },
        );
        functions.insert(
            METADATA_LOCK_IDENT.to_string(),
            FunctionSchemaInit {
                receiver: Some(ReceiverInfo::normal_ref_mut()),
                input: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<MetadataLockInput>(),
                ),
                output: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<MetadataLockOutput>(),
                ),
                export: METADATA_LOCK_IDENT.to_string(),
            },
        );
        functions.insert(
            METADATA_GET_IDENT.to_string(),
            FunctionSchemaInit {
                receiver: Some(ReceiverInfo::normal_ref()),
                input: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<MetadataGetInput>(),
                ),
                output: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<MetadataGetOutput>(),
                ),
                export: METADATA_GET_IDENT.to_string(),
            },
        );
        functions.insert(
            METADATA_REMOVE_IDENT.to_string(),
            FunctionSchemaInit {
                receiver: Some(ReceiverInfo::normal_ref_mut()),
                input: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<MetadataRemoveInput>(),
                ),
                output: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<MetadataRemoveOutput>(),
                ),
                export: METADATA_REMOVE_IDENT.to_string(),
            },
        );

        let events = event_schema! {
            aggregator,
            [SetMetadataEvent, RemoveMetadataEvent]
        };

        let schema = generate_full_schema(aggregator);
        let blueprints = btreemap!(
            METADATA_BLUEPRINT.to_string() => BlueprintDefinitionInit {
                blueprint_type: BlueprintType::default(),
                is_transient: true,
                feature_set: btreeset!(),
                dependencies: btreeset!(),

                schema: BlueprintSchemaInit {
                    generics: vec![],
                    schema,
                    state: BlueprintStateSchemaInit {
                        fields: vec![],
                        collections,
                    },
                    events,
                    functions: BlueprintFunctionsSchemaInit {
                        functions,
                    },
                    hooks: BlueprintHooksInit::default(),
                },

                royalty_config: PackageRoyaltyConfig::default(),
                auth_config: AuthConfig {
                    function_auth: FunctionAuth::AllowAll,
                    method_auth: MethodAuthTemplate::StaticRoleDefinition(
                        roles_template!(
                            roles {
                                METADATA_SETTER_ROLE => updaters: [METADATA_SETTER_UPDATER_ROLE];
                                METADATA_SETTER_UPDATER_ROLE => updaters: [METADATA_SETTER_UPDATER_ROLE];
                                METADATA_LOCKER_ROLE => updaters: [METADATA_LOCKER_UPDATER_ROLE];
                                METADATA_LOCKER_UPDATER_ROLE => updaters: [METADATA_LOCKER_UPDATER_ROLE];
                            },
                            methods {
                                METADATA_SET_IDENT => [METADATA_SETTER_ROLE];
                                METADATA_REMOVE_IDENT => [METADATA_SETTER_ROLE];
                                METADATA_LOCK_IDENT => [METADATA_LOCKER_ROLE];
                                METADATA_GET_IDENT => MethodAccessibility::Public;
                            }
                        ),
                    ),
                },
            }
        );

        PackageDefinition { blueprints }
    }

    pub fn invoke_export<Y>(
        export_name: &str,
        input: &IndexedScryptoValue,
        api: &mut Y,
    ) -> Result<IndexedScryptoValue, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        match export_name {
            METADATA_CREATE_IDENT => {
                let _input: MetadataCreateInput = input.as_typed().map_err(|e| {
                    RuntimeError::ApplicationError(ApplicationError::InputDecodeError(e))
                })?;

                let rtn = Self::create(api)?;

                Ok(IndexedScryptoValue::from_typed(&rtn))
            }
            METADATA_CREATE_WITH_DATA_IDENT => {
                let input: MetadataCreateWithDataInput = input.as_typed().map_err(|e| {
                    RuntimeError::ApplicationError(ApplicationError::InputDecodeError(e))
                })?;

                let rtn = Self::create_with_data(input.data, api)?;

                Ok(IndexedScryptoValue::from_typed(&rtn))
            }
            METADATA_SET_IDENT => {
                let input: MetadataSetInput = input.as_typed().map_err(|e| {
                    RuntimeError::ApplicationError(ApplicationError::InputDecodeError(e))
                })?;

                let rtn = Self::set(input.key, input.value, api)?;

                Ok(IndexedScryptoValue::from_typed(&rtn))
            }
            METADATA_LOCK_IDENT => {
                let input: MetadataLockInput = input.as_typed().map_err(|e| {
                    RuntimeError::ApplicationError(ApplicationError::InputDecodeError(e))
                })?;

                let rtn = Self::lock(input.key, api)?;

                Ok(IndexedScryptoValue::from_typed(&rtn))
            }
            METADATA_GET_IDENT => {
                let input: MetadataGetInput = input.as_typed().map_err(|e| {
                    RuntimeError::ApplicationError(ApplicationError::InputDecodeError(e))
                })?;

                let rtn = Self::get(input.key, api)?;

                Ok(IndexedScryptoValue::from_typed(&rtn))
            }
            METADATA_REMOVE_IDENT => {
                let input: MetadataRemoveInput = input.as_typed().map_err(|e| {
                    RuntimeError::ApplicationError(ApplicationError::InputDecodeError(e))
                })?;

                let rtn = Self::remove(input.key, api)?;
                Ok(IndexedScryptoValue::from_typed(&rtn))
            }
            _ => Err(RuntimeError::ApplicationError(
                ApplicationError::ExportDoesNotExist(export_name.to_string()),
            )),
        }
    }

    pub(crate) fn create<Y>(api: &mut Y) -> Result<Own, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let node_id = api.new_object(
            METADATA_BLUEPRINT,
            vec![],
            GenericArgs::default(),
            vec![],
            btreemap!(),
        )?;

        Ok(Own(node_id))
    }

    pub fn init_system_struct(data: MetadataInit) -> Result<(Vec<Option<FieldValue>>, BTreeMap<u8, BTreeMap<Vec<u8>, KVEntry>>), MetadataPanicError> {
        let mut init_kv_entries = BTreeMap::new();
        for (key, entry) in data.data {
            if key.len() > MAX_METADATA_KEY_STRING_LEN {
                return Err(MetadataPanicError::KeyStringExceedsMaxLength {
                            max: MAX_METADATA_KEY_STRING_LEN,
                            actual: key.len(),
                        },
                    );
            }

            let key = scrypto_encode(&key).unwrap();

            let value = match entry.value {
                Some(metadata_value) => {
                    let value = scrypto_encode(&metadata_value).unwrap();
                    if value.len() > MAX_METADATA_VALUE_SBOR_LEN {
                        return Err(MetadataPanicError::ValueSborExceedsMaxLength {
                                    max: MAX_METADATA_VALUE_SBOR_LEN,
                                    actual: value.len(),
                                },
                        );
                    }
                    Some(value)
                }
                None => None,
            };

            let kv_entry = KVEntry {
                value,
                locked: entry.lock,
            };

            init_kv_entries.insert(key, kv_entry);
        }

        Ok((vec![], btreemap!(METADATA_COLLECTION => init_kv_entries)))
    }

    pub(crate) fn create_with_data<Y>(data: MetadataInit, api: &mut Y) -> Result<Own, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let (fields, kv_entries) = Self::init_system_struct(data).map_err(|e| RuntimeError::ApplicationError(ApplicationError::MetadataError(e)))?;
        let node_id = api.new_object(
            METADATA_BLUEPRINT,
            vec![],
            GenericArgs::default(),
            fields.into_iter().map(|f| match f {
                Some(f) => f,
                None => FieldValue::new(()),
            }).collect(),
            kv_entries,
        )?;

        Ok(Own(node_id))
    }

    pub(crate) fn set<Y>(key: String, value: MetadataValue, api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        if key.len() > MAX_METADATA_KEY_STRING_LEN {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::MetadataError(MetadataPanicError::KeyStringExceedsMaxLength {
                    max: MAX_METADATA_KEY_STRING_LEN,
                    actual: key.len(),
                }),
            ));
        }

        let sbor_value = scrypto_encode(&value).unwrap();
        if sbor_value.len() > MAX_METADATA_VALUE_SBOR_LEN {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::MetadataError(MetadataPanicError::ValueSborExceedsMaxLength {
                    max: MAX_METADATA_VALUE_SBOR_LEN,
                    actual: sbor_value.len(),
                }),
            ));
        }

        let handle = api.actor_open_key_value_entry(
            OBJECT_HANDLE_SELF,
            0u8,
            &scrypto_encode(&key).unwrap(),
            LockFlags::MUTABLE,
        )?;
        api.key_value_entry_set(handle, sbor_value)?;
        api.key_value_entry_close(handle)?;

        Runtime::emit_event(api, SetMetadataEvent { key, value })?;

        Ok(())
    }

    pub(crate) fn lock<Y>(key: String, api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let handle = api.actor_open_key_value_entry(
            OBJECT_HANDLE_SELF,
            0u8,
            &scrypto_encode(&key).unwrap(),
            LockFlags::MUTABLE,
        )?;
        api.key_value_entry_lock(handle)?;
        api.key_value_entry_close(handle)?;

        Ok(())
    }

    pub(crate) fn get<Y>(key: String, api: &mut Y) -> Result<Option<MetadataValue>, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let handle = api.actor_open_key_value_entry(
            OBJECT_HANDLE_SELF,
            0u8,
            &scrypto_encode(&key).unwrap(),
            LockFlags::read_only(),
        )?;

        let data = api.key_value_entry_get(handle)?;
        let substate: Option<MetadataValue> = scrypto_decode(&data).unwrap();

        Ok(substate)
    }

    pub(crate) fn remove<Y>(key: String, api: &mut Y) -> Result<bool, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let cur_value: Option<MetadataValue> = api.actor_remove_key_value_entry_typed(
            OBJECT_HANDLE_SELF,
            0u8,
            &scrypto_encode(&key).unwrap(),
        )?;
        let rtn = cur_value.is_some();

        Runtime::emit_event(api, RemoveMetadataEvent { key })?;

        Ok(rtn)
    }
}
