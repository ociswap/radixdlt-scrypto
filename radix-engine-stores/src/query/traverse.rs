use super::TypeInfoSubstate;
use crate::interface::SubstateDatabase;
use radix_engine_interface::blueprints::resource::{
    LiquidNonFungibleVault, FUNGIBLE_VAULT_BLUEPRINT, NON_FUNGIBLE_VAULT_BLUEPRINT,
};
use radix_engine_interface::constants::RESOURCE_MANAGER_PACKAGE;
use radix_engine_interface::data::scrypto::scrypto_decode;
use radix_engine_interface::types::{
    FungibleVaultOffset, IndexedScryptoValue, IntoEnumIterator, ModuleId, NonFungibleVaultOffset,
    ObjectInfo, ResourceAddress, SubstateKey, SysModuleId,
};
use radix_engine_interface::{blueprints::resource::LiquidFungibleResource, types::NodeId};
use sbor::rust::prelude::*;

pub struct StateTreeTraverser<'s, 'v, S: SubstateDatabase, V: StateTreeVisitor> {
    substate_db: &'s S,
    visitor: &'v mut V,
    max_depth: u32,
}

pub trait StateTreeVisitor {
    fn visit_fungible_vault(
        &mut self,
        _vault_id: NodeId,
        _address: &ResourceAddress,
        _resource: &LiquidFungibleResource,
    ) {
    }

    fn visit_non_fungible_vault(
        &mut self,
        _vault_id: NodeId,
        _address: &ResourceAddress,
        _resource: &LiquidNonFungibleVault,
    ) {
    }

    fn visit_node_id(
        &mut self,
        _parent_id: Option<&(NodeId, ModuleId, SubstateKey)>,
        _node_id: &NodeId,
        _depth: u32,
    ) {
    }
}

impl<'s, 'v, S: SubstateDatabase, V: StateTreeVisitor> StateTreeTraverser<'s, 'v, S, V> {
    pub fn new(substate_db: &'s S, visitor: &'v mut V, max_depth: u32) -> Self {
        StateTreeTraverser {
            substate_db,
            visitor,
            max_depth,
        }
    }

    pub fn traverse_all_descendents(
        &mut self,
        parent_node_id: Option<&(NodeId, ModuleId, SubstateKey)>,
        node_id: NodeId,
    ) {
        self.traverse_recursive(parent_node_id, node_id, 0)
    }

    fn traverse_recursive(
        &mut self,
        parent: Option<&(NodeId, ModuleId, SubstateKey)>,
        node_id: NodeId,
        depth: u32,
    ) {
        if depth > self.max_depth {
            return;
        }

        // Notify visitor
        self.visitor.visit_node_id(parent, &node_id, depth);

        // Load type info
        let type_info: TypeInfoSubstate = scrypto_decode(
            &self
                .substate_db
                .get_substate(
                    &node_id,
                    SysModuleId::TypeInfo.into(),
                    &SubstateKey::from_vec(vec![0]).unwrap(),
                )
                .expect("Missing TypeInfo substate"),
        )
        .expect("Failed to decode TypeInfo substate");

        match type_info {
            TypeInfoSubstate::KeyValueStore(_) => {
                for (substate_key, value) in self
                    .substate_db
                    .list_substates(&node_id, SysModuleId::VirtualizedObject.into())
                {
                    let (_, owned_nodes, _) = IndexedScryptoValue::from_vec(value)
                        .expect("Substate is not a scrypto value")
                        .unpack();
                    for child_node_id in owned_nodes {
                        self.traverse_recursive(
                            Some(&(
                                node_id,
                                SysModuleId::VirtualizedObject.into(),
                                substate_key.clone(),
                            )),
                            child_node_id,
                            depth + 1,
                        );
                    }
                }
            }
            TypeInfoSubstate::IterableStore | TypeInfoSubstate::SortedStore => {}
            TypeInfoSubstate::Object(ObjectInfo {
                blueprint,
                type_parent,
                global: _,
            }) => {
                if blueprint.package_address.eq(&RESOURCE_MANAGER_PACKAGE)
                    && blueprint.blueprint_name.eq(FUNGIBLE_VAULT_BLUEPRINT)
                {
                    let liquid: LiquidFungibleResource = scrypto_decode(
                        &self
                            .substate_db
                            .get_substate(
                                &node_id,
                                SysModuleId::Object.into(),
                                &FungibleVaultOffset::LiquidFungible.into(),
                            )
                            .expect("Broken database"),
                    )
                    .expect("Failed to decode liquid fungible");

                    self.visitor.visit_fungible_vault(
                        node_id.into(),
                        &ResourceAddress::new_unchecked(type_parent.unwrap().into()),
                        &liquid,
                    );
                } else if blueprint.package_address.eq(&RESOURCE_MANAGER_PACKAGE)
                    && blueprint.blueprint_name.eq(NON_FUNGIBLE_VAULT_BLUEPRINT)
                {
                    let liquid: LiquidNonFungibleVault = scrypto_decode(
                        &self
                            .substate_db
                            .get_substate(
                                &node_id,
                                SysModuleId::Object.into(),
                                &NonFungibleVaultOffset::LiquidNonFungible.into(),
                            )
                            .expect("Broken database"),
                    )
                    .expect("Failed to decode liquid non-fungible");

                    self.visitor.visit_non_fungible_vault(
                        node_id.into(),
                        &ResourceAddress::new_unchecked(type_parent.unwrap().into()),
                        &liquid,
                    );
                } else {
                    for t in SysModuleId::iter() {
                        // List all iterable modules (currently `ObjectState` & `Metadata`)
                        let x = self.substate_db.list_substates(&node_id, t.into());
                        for (substate_key, substate_value) in x {
                            let (_, owned_nodes, _) = IndexedScryptoValue::from_vec(substate_value)
                                .expect("Substate is not a scrypto value")
                                .unpack();
                            for child_node_id in owned_nodes {
                                self.traverse_recursive(
                                    Some(&(
                                        node_id,
                                        SysModuleId::Object.into(),
                                        substate_key.clone(),
                                    )),
                                    child_node_id,
                                    depth + 1,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}