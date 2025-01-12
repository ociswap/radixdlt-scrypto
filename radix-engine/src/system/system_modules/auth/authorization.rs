use crate::blueprints::resource::AuthZone;
use crate::errors::RuntimeError;
use crate::kernel::kernel_api::KernelSubstateApi;
use crate::system::node_modules::role_assignment::{
    RoleAssignmentAccessRuleEntryPayload, RoleAssignmentOwnerFieldPayload,
};
use crate::system::system_modules::auth::{
    AuthorityListAuthorizationResult, AuthorizationCheckResult,
};
use crate::system::system_substates::FieldSubstate;
use crate::system::system_substates::KeyValueEntrySubstate;
use crate::types::*;
use native_sdk::resource::{NativeNonFungibleProof, NativeProof};
use radix_engine_interface::api::{ClientObjectApi, LockFlags, ObjectModuleId};
use radix_engine_interface::blueprints::resource::*;
use sbor::rust::ops::Fn;

pub struct Authorization;

impl Authorization {
    fn proof_matches<L: Default, Y: KernelSubstateApi<L> + ClientObjectApi<RuntimeError>>(
        resource_rule: &ResourceOrNonFungible,
        proof: &Proof,
        api: &mut Y,
    ) -> Result<bool, RuntimeError> {
        match resource_rule {
            ResourceOrNonFungible::NonFungible(non_fungible_global_id) => {
                let proof_resource_address = proof.resource_address(api)?;
                Ok(
                    proof_resource_address == non_fungible_global_id.resource_address()
                        && proof
                            .non_fungible_local_ids(api)?
                            .contains(non_fungible_global_id.local_id()),
                )
            }
            ResourceOrNonFungible::Resource(resource_address) => {
                let proof_resource_address = proof.resource_address(api)?;
                Ok(proof_resource_address == *resource_address)
            }
        }
    }

    fn global_auth_zone_matches<L: Default, Y, P>(
        api: &mut Y,
        auth_zone_id: &NodeId,
        check: &P,
    ) -> Result<bool, RuntimeError>
    where
        Y: KernelSubstateApi<L>,
        P: Fn(
            &[Proof],
            &BTreeSet<ResourceAddress>,
            BTreeSet<NonFungibleGlobalId>,
            &mut Y,
        ) -> Result<bool, RuntimeError>,
    {
        let mut pass = false;
        let mut current_auth_zone_id = *auth_zone_id;
        let mut handles = Vec::new();
        loop {
            // Load auth zone
            let handle = api.kernel_open_substate(
                &current_auth_zone_id,
                MAIN_BASE_PARTITION,
                &AuthZoneField::AuthZone.into(),
                LockFlags::read_only(),
                L::default(),
            )?;
            let auth_zone = api
                .kernel_read_substate(handle)?
                .as_typed::<FieldSubstate<AuthZone>>()
                .unwrap()
                .into_payload();
            handles.push(handle);

            {
                let mut virtual_non_fungible_global_ids = BTreeSet::new();
                let virtual_resources = auth_zone.virtual_resources();

                virtual_non_fungible_global_ids.extend(auth_zone.virtual_non_fungibles().clone());

                let proofs = auth_zone.proofs();

                // Check
                if check(
                    proofs,
                    virtual_resources,
                    virtual_non_fungible_global_ids,
                    api,
                )? {
                    pass = true;
                    break;
                }
            }

            if let Some(id) = auth_zone.parent {
                current_auth_zone_id = id.into();
            } else {
                break;
            }
        }

        for handle in handles {
            api.kernel_close_substate(handle)?;
        }

        Ok(pass)
    }

    fn auth_zone_stack_matches<P, L, Y>(
        auth_zone: &NodeId,
        api: &mut Y,
        check: P,
    ) -> Result<bool, RuntimeError>
    where
        L: Default,
        Y: KernelSubstateApi<L>,
        P: Fn(
            &[Proof],
            &BTreeSet<ResourceAddress>,
            BTreeSet<NonFungibleGlobalId>,
            &mut Y,
        ) -> Result<bool, RuntimeError>,
    {
        let handle = api.kernel_open_substate(
            &auth_zone,
            MAIN_BASE_PARTITION,
            &AuthZoneField::AuthZone.into(),
            LockFlags::read_only(),
            L::default(),
        )?;

        // Using this block structure to be able to ensure handle is freed
        // The suggested Rust pattern seems to be to use RAII pattern + Drop but
        // at the moment this does not seem practical to be able to implement
        let rtn = (|| -> Result<bool, RuntimeError> {
            let auth_zone = api
                .kernel_read_substate(handle)?
                .as_typed::<FieldSubstate<AuthZone>>()
                .unwrap()
                .into_payload();

            // Check Local virtual non fungibles
            let virtual_proofs = auth_zone.local_virtual_non_fungibles();
            if !virtual_proofs.is_empty() {
                if check(&[], &btreeset!(), virtual_proofs, api)? {
                    return Ok(true);
                }
            }

            // Check global caller's full auth zone
            if let Some((_global_caller, global_caller_reference)) = &auth_zone.global_caller {
                if Self::global_auth_zone_matches(api, &global_caller_reference.0, &check)? {
                    return Ok(true);
                }
            }

            // Check current caller's full auth zone
            // We ignore the current frame's authzone since it is not relevant
            if let Some(parent) = auth_zone.parent {
                if Self::global_auth_zone_matches(api, &parent.0, &check)? {
                    return Ok(true);
                }
            }

            Ok(false)
        })()?;

        api.kernel_close_substate(handle)?;

        Ok(rtn)
    }

    fn auth_zone_stack_has_amount<
        Y: KernelSubstateApi<L> + ClientObjectApi<RuntimeError>,
        L: Default,
    >(
        auth_zone: &NodeId,
        resource: &ResourceAddress,
        amount: Decimal,
        api: &mut Y,
    ) -> Result<bool, RuntimeError> {
        Self::auth_zone_stack_matches(auth_zone, api, |proofs, _, _, api| {
            // TODO: revisit this and decide if we need to check the composite max amount rather than just each proof individually
            for p in proofs {
                if Self::proof_matches(&ResourceOrNonFungible::Resource(*resource), p, api)?
                    && p.amount(api)? >= amount
                {
                    return Ok(true);
                }
            }

            Ok(false)
        })
    }

    fn auth_zone_stack_matches_rule<
        Y: KernelSubstateApi<L> + ClientObjectApi<RuntimeError>,
        L: Default,
    >(
        auth_zone: &NodeId,
        resource_rule: &ResourceOrNonFungible,
        api: &mut Y,
    ) -> Result<bool, RuntimeError> {
        Self::auth_zone_stack_matches(
            auth_zone,
            api,
            |proofs, virtual_resources, virtual_non_fungibles, api| {
                if let ResourceOrNonFungible::NonFungible(non_fungible_global_id) = resource_rule {
                    if virtual_non_fungibles.contains(non_fungible_global_id) {
                        return Ok(true);
                    }

                    if virtual_resources.contains(&non_fungible_global_id.resource_address()) {
                        return Ok(true);
                    }
                }

                for p in proofs {
                    if Self::proof_matches(resource_rule, p, api)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            },
        )
    }

    pub fn verify_proof_rule<
        Y: KernelSubstateApi<L> + ClientObjectApi<RuntimeError>,
        L: Default,
    >(
        auth_zone: &NodeId,
        proof_rule: &ProofRule,
        api: &mut Y,
    ) -> Result<bool, RuntimeError> {
        match proof_rule {
            ProofRule::Require(resource) => {
                if Self::auth_zone_stack_matches_rule(auth_zone, resource, api)? {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            ProofRule::AmountOf(amount, resource) => {
                if Self::auth_zone_stack_has_amount(auth_zone, resource, *amount, api)? {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            ProofRule::AllOf(resources) => {
                for resource in resources {
                    if !Self::auth_zone_stack_matches_rule(auth_zone, resource, api)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            ProofRule::AnyOf(resources) => {
                for resource in resources {
                    if Self::auth_zone_stack_matches_rule(auth_zone, resource, api)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }
            ProofRule::CountOf(count, resources) => {
                let mut left = count.clone();
                for resource in resources {
                    if Self::auth_zone_stack_matches_rule(auth_zone, resource, api)? {
                        left -= 1;
                        if left == 0 {
                            return Ok(true);
                        }
                    }
                }
                Ok(false)
            }
        }
    }

    pub fn verify_auth_rule<Y: KernelSubstateApi<L> + ClientObjectApi<RuntimeError>, L: Default>(
        auth_zone: &NodeId,
        auth_rule: &AccessRuleNode,
        api: &mut Y,
    ) -> Result<AuthorizationCheckResult, RuntimeError> {
        match auth_rule {
            AccessRuleNode::ProofRule(rule) => {
                if Self::verify_proof_rule(auth_zone, rule, api)? {
                    Ok(AuthorizationCheckResult::Authorized)
                } else {
                    Ok(AuthorizationCheckResult::Failed(vec![]))
                }
            }
            AccessRuleNode::AnyOf(rules) => {
                for r in rules {
                    let rtn = Self::verify_auth_rule(auth_zone, r, api)?;
                    if matches!(rtn, AuthorizationCheckResult::Authorized) {
                        return Ok(rtn);
                    }
                }
                Ok(AuthorizationCheckResult::Failed(vec![]))
            }
            AccessRuleNode::AllOf(rules) => {
                for r in rules {
                    let rtn = Self::verify_auth_rule(auth_zone, r, api)?;
                    if matches!(rtn, AuthorizationCheckResult::Failed(..)) {
                        return Ok(rtn);
                    }
                }

                return Ok(AuthorizationCheckResult::Authorized);
            }
        }
    }

    pub fn check_authorization_against_role_key_internal<
        Y: KernelSubstateApi<L> + ClientObjectApi<RuntimeError>,
        L: Default,
    >(
        auth_zone: &NodeId,
        role_assignment_of: &GlobalAddress,
        key: &ModuleRoleKey,
        api: &mut Y,
    ) -> Result<AuthorizationCheckResult, RuntimeError> {
        let access_rule = if key.key.key.eq(SELF_ROLE) {
            rule!(require(global_caller(role_assignment_of.clone())))
        } else {
            let handle = api.kernel_open_substate_with_default(
                role_assignment_of.as_node_id(),
                ROLE_ASSIGNMENT_BASE_PARTITION
                    .at_offset(ROLE_ASSIGNMENT_ROLE_DEF_PARTITION_OFFSET)
                    .unwrap(),
                &SubstateKey::Map(scrypto_encode(&key).unwrap()),
                LockFlags::read_only(),
                Some(|| {
                    let kv_entry = KeyValueEntrySubstate::<()>::default();
                    IndexedScryptoValue::from_typed(&kv_entry)
                }),
                L::default(),
            )?;
            let substate: KeyValueEntrySubstate<RoleAssignmentAccessRuleEntryPayload> =
                api.kernel_read_substate(handle)?.as_typed().unwrap();
            api.kernel_close_substate(handle)?;

            match substate.into_value() {
                Some(access_rule) => access_rule.content.into_latest(),
                None => {
                    let handle = api.kernel_open_substate(
                        role_assignment_of.as_node_id(),
                        ROLE_ASSIGNMENT_BASE_PARTITION
                            .at_offset(ROLE_ASSIGNMENT_FIELDS_PARTITION_OFFSET)
                            .unwrap(),
                        &SubstateKey::Field(0u8),
                        LockFlags::read_only(),
                        L::default(),
                    )?;

                    let owner_role_substate: FieldSubstate<RoleAssignmentOwnerFieldPayload> =
                        api.kernel_read_substate(handle)?.as_typed().unwrap();
                    api.kernel_close_substate(handle)?;
                    owner_role_substate
                        .into_payload()
                        .into_latest()
                        .owner_role_entry
                        .rule
                }
            }
        };

        Self::check_authorization_against_access_rule(api, auth_zone, &access_rule)
    }

    pub fn check_authorization_against_access_rule<
        Y: KernelSubstateApi<L> + ClientObjectApi<RuntimeError>,
        L: Default,
    >(
        api: &mut Y,
        auth_zone: &NodeId,
        rule: &AccessRule,
    ) -> Result<AuthorizationCheckResult, RuntimeError> {
        match rule {
            AccessRule::Protected(rule_node) => {
                let mut rtn = Self::verify_auth_rule(auth_zone, rule_node, api)?;
                match &mut rtn {
                    AuthorizationCheckResult::Authorized => {}
                    AuthorizationCheckResult::Failed(stack) => {
                        stack.push(rule.clone());
                    }
                }
                Ok(rtn)
            }
            AccessRule::AllowAll => Ok(AuthorizationCheckResult::Authorized),
            AccessRule::DenyAll => Ok(AuthorizationCheckResult::Failed(vec![rule.clone()])),
        }
    }

    pub fn check_authorization_against_role_list<
        Y: KernelSubstateApi<L> + ClientObjectApi<RuntimeError>,
        L: Default,
    >(
        auth_zone: &NodeId,
        role_assignment_of: &GlobalAddress,
        module: ObjectModuleId,
        role_list: &RoleList,
        api: &mut Y,
    ) -> Result<AuthorityListAuthorizationResult, RuntimeError> {
        let mut failed = Vec::new();

        for key in &role_list.list {
            let module_role_key = ModuleRoleKey::new(module, key.key.as_str());
            let result = Self::check_authorization_against_role_key_internal(
                &auth_zone,
                role_assignment_of,
                &module_role_key,
                api,
            )?;
            match result {
                AuthorizationCheckResult::Authorized => {
                    return Ok(AuthorityListAuthorizationResult::Authorized)
                }
                AuthorizationCheckResult::Failed(stack) => {
                    failed.push((key.clone(), stack));
                }
            }
        }

        Ok(AuthorityListAuthorizationResult::Failed(failed))
    }
}
