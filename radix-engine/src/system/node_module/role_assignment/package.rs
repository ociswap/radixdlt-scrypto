use crate::blueprints::package::PackageAuthNativeBlueprint;
use crate::kernel::kernel_api::{KernelApi, KernelSubstateApi};
use crate::system::node_module::role_assignment::{LockOwnerRoleEvent, SetOwnerRoleEvent};
use crate::system::system::{FieldSubstate, SystemService};
use crate::system::system_callback::{SystemConfig, SystemLockData};
use crate::system::system_callback_api::SystemCallbackObject;
use crate::system::system_modules::auth::{AuthError, ResolvedPermission};
use crate::types::*;
use crate::{errors::*, event_schema};
use native_sdk::runtime::Runtime;
use radix_engine_interface::api::field_api::LockFlags;
use radix_engine_interface::api::node_modules::auth::*;
use radix_engine_interface::api::{
    ClientApi, FieldValue, GenericArgs, KVEntry, ObjectModuleId, OBJECT_HANDLE_SELF,
};
use radix_engine_interface::blueprints::package::{
    AuthConfig, BlueprintDefinitionInit, BlueprintType, BlueprintVersionKey, FunctionAuth,
    MethodAuthTemplate, PackageDefinition, RoleSpecification,
};
use radix_engine_interface::blueprints::resource::*;
use radix_engine_interface::schema::{
    BlueprintCollectionSchema, BlueprintFunctionsSchemaInit, BlueprintSchemaInit,
    BlueprintStateSchemaInit, FieldSchema, FunctionSchemaInit, TypeRef,
};
use radix_engine_interface::types::*;

use super::SetRoleEvent;

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub enum RoleAssignmentError {
    UsedReservedRole(String),
    UsedReservedSpace,
    ExceededMaxRoleNameLen { limit: usize, actual: usize },
    ExceededMaxAccessRuleDepth,
    ExceededMaxAccessRuleNodes,
}

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
#[sbor(transparent)]
pub struct OwnerRoleSubstate {
    pub owner_role_entry: OwnerRoleEntry,
}

pub struct RoleAssignmentNativePackage;

impl RoleAssignmentNativePackage {
    pub fn definition() -> PackageDefinition {
        let mut aggregator = TypeAggregator::<ScryptoCustomTypeKind>::new();

        let mut fields = Vec::new();
        fields.push(FieldSchema::static_field(
            aggregator.add_child_type_and_descendents::<OwnerRoleSubstate>(),
        ));

        let mut collections = Vec::new();
        collections.push(BlueprintCollectionSchema::KeyValueStore(
            BlueprintKeyValueSchema {
                key: TypeRef::Static(aggregator.add_child_type_and_descendents::<ModuleRoleKey>()),
                value: TypeRef::Static(aggregator.add_child_type_and_descendents::<AccessRule>()),
                allow_ownership: false,
            },
        ));

        let mut functions = BTreeMap::new();
        functions.insert(
            ROLE_ASSIGNMENT_CREATE_IDENT.to_string(),
            FunctionSchemaInit {
                receiver: None,
                input: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<RoleAssignmentCreateInput>(),
                ),
                output: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<RoleAssignmentCreateOutput>(),
                ),
                export: ROLE_ASSIGNMENT_CREATE_IDENT.to_string(),
            },
        );
        functions.insert(
            ROLE_ASSIGNMENT_SET_OWNER_IDENT.to_string(),
            FunctionSchemaInit {
                receiver: Some(ReceiverInfo::normal_ref_mut()),
                input: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<RoleAssignmentSetOwnerInput>(),
                ),
                output: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<RoleAssignmentSetOwnerOutput>(),
                ),
                export: ROLE_ASSIGNMENT_SET_OWNER_IDENT.to_string(),
            },
        );
        functions.insert(
            ROLE_ASSIGNMENT_LOCK_OWNER_IDENT.to_string(),
            FunctionSchemaInit {
                receiver: Some(ReceiverInfo::normal_ref_mut()),
                input: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<RoleAssignmentLockOwnerInput>(),
                ),
                output: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<RoleAssingmentLockOwnerOutput>(),
                ),
                export: ROLE_ASSIGNMENT_LOCK_OWNER_IDENT.to_string(),
            },
        );
        functions.insert(
            ROLE_ASSIGNMENT_SET_IDENT.to_string(),
            FunctionSchemaInit {
                receiver: Some(ReceiverInfo::normal_ref_mut()),
                input: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<RoleAssignmentSetInput>(),
                ),
                output: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<RoleAssignmentSetRoleOutput>(),
                ),
                export: ROLE_ASSIGNMENT_SET_IDENT.to_string(),
            },
        );
        functions.insert(
            ROLE_ASSIGNMENT_GET_IDENT.to_string(),
            FunctionSchemaInit {
                receiver: Some(ReceiverInfo::normal_ref_mut()),
                input: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<RoleAssignmentGetInput>(),
                ),
                output: TypeRef::Static(
                    aggregator.add_child_type_and_descendents::<RoleAssignmentGetOutput>(),
                ),
                export: ROLE_ASSIGNMENT_GET_IDENT.to_string(),
            },
        );

        let events = event_schema! {
            aggregator,
            [
                SetOwnerRoleEvent,
                SetRoleEvent,
                LockOwnerRoleEvent
            ]
        };

        let schema = generate_full_schema(aggregator);
        let blueprints = btreemap!(
            ROLE_ASSIGNMENT_BLUEPRINT.to_string() => BlueprintDefinitionInit {
                blueprint_type: BlueprintType::default(),
                is_transient: true,
                feature_set: btreeset!(),
                dependencies: btreeset!(),

                schema: BlueprintSchemaInit {
                    generics: vec![],
                    schema,
                    state: BlueprintStateSchemaInit {
                        fields,
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
                    method_auth: MethodAuthTemplate::AllowAll, // Mocked
                },
            }
        );

        PackageDefinition { blueprints }
    }

    pub fn authorization<Y: KernelApi<SystemConfig<V>>, V: SystemCallbackObject>(
        global_address: &GlobalAddress,
        ident: &str,
        input: &IndexedScryptoValue,
        api: &mut SystemService<Y, V>,
    ) -> Result<ResolvedPermission, RuntimeError> {
        let permission = match ident {
            ROLE_ASSIGNMENT_SET_IDENT => {
                let input: RoleAssignmentSetInput = input.as_typed().map_err(|e| {
                    RuntimeError::ApplicationError(ApplicationError::InputDecodeError(e))
                })?;
                let role_list = Self::resolve_update_role_method_permission(
                    global_address.as_node_id(),
                    input.module,
                    &input.role_key,
                    api,
                )?;
                ResolvedPermission::RoleList {
                    role_assignment_of: global_address.clone(),
                    role_list,
                    module_id: input.module,
                }
            }
            ROLE_ASSIGNMENT_SET_OWNER_IDENT => {
                Self::resolve_update_owner_role_method_permission(global_address.as_node_id(), api)?
            }
            ROLE_ASSIGNMENT_LOCK_OWNER_IDENT => {
                Self::resolve_update_owner_role_method_permission(global_address.as_node_id(), api)?
            }
            ROLE_ASSIGNMENT_GET_IDENT => ResolvedPermission::AllowAll,
            _ => {
                return Err(RuntimeError::SystemModuleError(
                    SystemModuleError::AuthError(AuthError::NoMethodMapping(FnIdentifier {
                        blueprint_id: BlueprintId::new(
                            &ROLE_ASSIGNMENT_MODULE_PACKAGE,
                            ROLE_ASSIGNMENT_BLUEPRINT,
                        ),
                        ident: ident.to_string(),
                    })),
                ));
            }
        };

        Ok(permission)
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
            ROLE_ASSIGNMENT_CREATE_IDENT => {
                let input: RoleAssignmentCreateInput = input.as_typed().map_err(|e| {
                    RuntimeError::ApplicationError(ApplicationError::InputDecodeError(e))
                })?;

                let rtn = Self::create(input.owner_role, input.roles, api)?;
                Ok(IndexedScryptoValue::from_typed(&rtn))
            }
            ROLE_ASSIGNMENT_SET_OWNER_IDENT => {
                let input: RoleAssignmentSetOwnerInput = input.as_typed().map_err(|e| {
                    RuntimeError::ApplicationError(ApplicationError::InputDecodeError(e))
                })?;

                let rtn = Self::set_owner_role(input.rule, api)?;
                Ok(IndexedScryptoValue::from_typed(&rtn))
            }
            ROLE_ASSIGNMENT_LOCK_OWNER_IDENT => {
                let _input: RoleAssignmentLockOwnerInput = input.as_typed().map_err(|e| {
                    RuntimeError::ApplicationError(ApplicationError::InputDecodeError(e))
                })?;

                let rtn = Self::lock_owner_role(api)?;
                Ok(IndexedScryptoValue::from_typed(&rtn))
            }
            ROLE_ASSIGNMENT_SET_IDENT => {
                let input: RoleAssignmentSetInput = input.as_typed().map_err(|e| {
                    RuntimeError::ApplicationError(ApplicationError::InputDecodeError(e))
                })?;

                let rtn = Self::set_role(input.module, input.role_key, input.rule, api)?;
                Ok(IndexedScryptoValue::from_typed(&rtn))
            }
            ROLE_ASSIGNMENT_GET_IDENT => {
                let input: RoleAssignmentGetInput = input.as_typed().map_err(|e| {
                    RuntimeError::ApplicationError(ApplicationError::InputDecodeError(e))
                })?;

                let rtn = Self::get_role(input.module, input.role_key, api)?;
                Ok(IndexedScryptoValue::from_typed(&rtn))
            }
            _ => Err(RuntimeError::ApplicationError(
                ApplicationError::ExportDoesNotExist(export_name.to_string()),
            )),
        }
    }

    pub fn is_reserved_role_key(role_key: &RoleKey) -> bool {
        return role_key.key.starts_with("_");
    }

    pub fn verify_access_rule(access_rule: &AccessRule) -> Result<(), RoleAssignmentError> {
        pub struct AccessRuleVerifier(usize);
        impl AccessRuleVisitor for AccessRuleVerifier {
            type Error = RoleAssignmentError;
            fn visit(&mut self, _node: &AccessRuleNode, depth: usize) -> Result<(), Self::Error> {
                // This is to protect unbounded native stack useage during authorization
                if depth > MAX_ACCESS_RULE_DEPTH {
                    return Err(RoleAssignmentError::ExceededMaxAccessRuleDepth);
                }

                self.0 += 1;

                if self.0 > MAX_ACCESS_RULE_NODES {
                    return Err(RoleAssignmentError::ExceededMaxAccessRuleNodes);
                }

                Ok(())
            }
        }

        access_rule.dfs_traverse_nodes(&mut AccessRuleVerifier(0))
    }

    fn resolve_update_owner_role_method_permission<
        Y: KernelApi<SystemConfig<V>>,
        V: SystemCallbackObject,
    >(
        receiver: &NodeId,
        api: &mut SystemService<Y, V>,
    ) -> Result<ResolvedPermission, RuntimeError> {
        let handle = api.kernel_open_substate(
            receiver,
            ROLE_ASSIGNMENT_BASE_PARTITION
                .at_offset(ROLE_ASSIGNMENT_FIELDS_PARTITION_OFFSET)
                .unwrap(),
            &SubstateKey::Field(0u8),
            LockFlags::read_only(),
            SystemLockData::default(),
        )?;

        let owner_role_substate: FieldSubstate<OwnerRoleSubstate> =
            api.kernel_read_substate(handle)?.as_typed().unwrap();
        api.kernel_close_substate(handle)?;

        let rule = match owner_role_substate.value.0.owner_role_entry.updater {
            OwnerRoleUpdater::None => AccessRule::DenyAll,
            OwnerRoleUpdater::Owner => owner_role_substate.value.0.owner_role_entry.rule,
            OwnerRoleUpdater::Object => rule!(require(global_caller(GlobalAddress::new_or_panic(
                receiver.0
            )))),
        };

        Ok(ResolvedPermission::AccessRule(rule))
    }

    fn resolve_update_role_method_permission<
        Y: KernelApi<SystemConfig<V>>,
        V: SystemCallbackObject,
    >(
        receiver: &NodeId,
        module: ObjectModuleId,
        role_key: &RoleKey,
        api: &mut SystemService<Y, V>,
    ) -> Result<RoleList, RuntimeError> {
        if Self::is_reserved_role_key(&role_key) {
            return Ok(RoleList::none());
        }

        let blueprint_id = api.get_blueprint_info(receiver, module)?.blueprint_id;

        let auth_template = PackageAuthNativeBlueprint::get_bp_auth_template(
            blueprint_id.package_address.as_node_id(),
            &BlueprintVersionKey::new_default(blueprint_id.blueprint_name.as_str()),
            api.api,
        )?
        .method_auth;

        match auth_template {
            MethodAuthTemplate::AllowAll => Ok(RoleList::none()),
            MethodAuthTemplate::StaticRoleDefinition(roles) => match roles.roles {
                RoleSpecification::Normal(roles) => match roles.get(&role_key) {
                    Some(role_list) => Ok(role_list.clone()),
                    None => Ok(RoleList::none()),
                },
                RoleSpecification::UseOuter => Ok(RoleList::none()),
            },
        }
    }

    pub fn init_system_struct(owner_role: OwnerRoleEntry, roles: BTreeMap<ObjectModuleId, RoleAssignmentInit>)
        -> Result<(Vec<Option<FieldValue>>, BTreeMap<u8, BTreeMap<Vec<u8>, KVEntry>>), RoleAssignmentError> {
        if roles.contains_key(&ObjectModuleId::RoleAssignment) {
            return Err(RoleAssignmentError::UsedReservedSpace);
        }

        Self::verify_access_rule(&owner_role.rule)?;

        let owner_role_substate = OwnerRoleSubstate {
            owner_role_entry: owner_role.clone(),
        };

        let owner_role_field = match owner_role.updater {
            OwnerRoleUpdater::None => FieldValue::immutable(&owner_role_substate),
            OwnerRoleUpdater::Owner | OwnerRoleUpdater::Object => {
                FieldValue::new(&owner_role_substate)
            }
        };

        let mut role_entries = BTreeMap::new();

        for (module, roles) in roles {
            for (role_key, role_def) in roles.data {
                if Self::is_reserved_role_key(&role_key) {
                    return Err(RoleAssignmentError::UsedReservedRole(role_key.key.to_string()));
                }

                if role_key.key.len() > MAX_ROLE_NAME_LEN {
                    return Err(RoleAssignmentError::ExceededMaxRoleNameLen {
                                limit: MAX_ROLE_NAME_LEN,
                                actual: role_key.key.len(),
                            }
                    );
                }

                let module_role_key = ModuleRoleKey::new(module, role_key);

                if let Some(access_rule) = &role_def {
                    Self::verify_access_rule(access_rule)?;
                }

                let value = role_def.map(|rule| scrypto_encode(&rule).unwrap());

                let kv_entry = KVEntry {
                    value,
                    locked: false,
                };

                role_entries.insert(scrypto_encode(&module_role_key).unwrap(), kv_entry);
            }
        }

        Ok((vec![Some(owner_role_field)], btreemap!(0 => role_entries)))
    }

    pub(crate) fn create<Y>(
        owner_role: OwnerRoleEntry,
        roles: BTreeMap<ObjectModuleId, RoleAssignmentInit>,
        api: &mut Y,
    ) -> Result<Own, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let (fields, kv_entries) = Self::init_system_struct(owner_role, roles)
            .map_err(|e| RuntimeError::ApplicationError(ApplicationError::RoleAssignmentError(e)))?;

        let component_id = api.new_object(
            ROLE_ASSIGNMENT_BLUEPRINT,
            vec![],
            GenericArgs::default(),
            fields.into_iter().map(|f| match f {
                Some(f) => f,
                None => FieldValue::new(()),
            }).collect(),
            kv_entries,
        )?;

        Ok(Own(component_id))
    }

    fn set_owner_role<Y>(rule: AccessRule, api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        Self::verify_access_rule(&rule).map_err(|e| {
            RuntimeError::ApplicationError(ApplicationError::RoleAssignmentError(e))
        })?;

        let handle = api.actor_open_field(OBJECT_HANDLE_SELF, 0u8, LockFlags::MUTABLE)?;

        let mut owner_role: OwnerRoleSubstate = api.field_read_typed(handle)?;
        owner_role.owner_role_entry.rule = rule.clone();
        api.field_write_typed(handle, owner_role)?;
        api.field_close(handle)?;

        Runtime::emit_event(api, SetOwnerRoleEvent { rule })?;

        Ok(())
    }

    fn lock_owner_role<Y>(api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let handle = api.actor_open_field(OBJECT_HANDLE_SELF, 0u8, LockFlags::MUTABLE)?;
        let mut owner_role: OwnerRoleSubstate = api.field_read_typed(handle)?;
        owner_role.owner_role_entry.updater = OwnerRoleUpdater::None;
        api.field_lock(handle)?;
        api.field_close(handle)?;

        Runtime::emit_event(api, LockOwnerRoleEvent {})?;

        Ok(())
    }

    fn set_role<Y>(
        module: ObjectModuleId,
        role_key: RoleKey,
        rule: AccessRule,
        api: &mut Y,
    ) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        if Self::is_reserved_role_key(&role_key) {
            if !module.eq(&ObjectModuleId::RoleAssignment) {
                return Err(RuntimeError::ApplicationError(
                    ApplicationError::RoleAssignmentError(RoleAssignmentError::UsedReservedRole(
                        role_key.key.to_string(),
                    )),
                ));
            }
        }

        if role_key.key.len() > MAX_ROLE_NAME_LEN {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::RoleAssignmentError(
                    RoleAssignmentError::ExceededMaxRoleNameLen {
                        limit: MAX_ROLE_NAME_LEN,
                        actual: role_key.key.len(),
                    },
                ),
            ));
        }

        let module_role_key = ModuleRoleKey::new(module, role_key.clone());

        Self::verify_access_rule(&rule).map_err(|e| {
            RuntimeError::ApplicationError(ApplicationError::RoleAssignmentError(e))
        })?;

        let handle = api.actor_open_key_value_entry(
            OBJECT_HANDLE_SELF,
            0u8,
            &scrypto_encode(&module_role_key).unwrap(),
            LockFlags::MUTABLE,
        )?;

        // Overwrite whatever access rule (or empty) is there
        api.key_value_entry_set_typed(handle, rule.clone())?;

        api.key_value_entry_set_typed(handle, rule.clone())?;
        api.key_value_entry_close(handle)?;

        Runtime::emit_event(api, SetRoleEvent { role_key, rule })?;

        Ok(())
    }

    pub(crate) fn get_role<Y>(
        module: ObjectModuleId,
        role_key: RoleKey,
        api: &mut Y,
    ) -> Result<Option<AccessRule>, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let module_role_key = ModuleRoleKey::new(module, role_key);

        let handle = api.actor_open_key_value_entry(
            OBJECT_HANDLE_SELF,
            0u8,
            &scrypto_encode(&module_role_key).unwrap(),
            LockFlags::read_only(),
        )?;

        let rule: Option<AccessRule> = api.key_value_entry_get_typed(handle)?;

        api.key_value_entry_close(handle)?;

        Ok(rule)
    }
}
