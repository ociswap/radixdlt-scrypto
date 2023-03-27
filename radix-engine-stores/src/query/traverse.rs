use crate::blueprints::resource::VaultInfoSubstate;
use crate::ledger::*;
use crate::system::node_modules::type_info::TypeInfoSubstate;
use crate::system::node_substates::PersistedSubstate;
use crate::types::*;
use radix_engine_interface::blueprints::resource::{
    LiquidFungibleResource, LiquidNonFungibleResource, ResourceType, VAULT_BLUEPRINT,
};
use radix_engine_interface::constants::RESOURCE_MANAGER_PACKAGE;

#[derive(Debug)]
pub enum StateTreeTraverserError {
    RENodeNotFound(NodeId),
    MaxDepthExceeded,
}

pub struct StateTreeTraverser<'s, 'v, S: SubstateDatabase, V: StateTreeVisitor> {
    substate_db: &'s S,
    visitor: &'v mut V,
    max_depth: u32,
}

pub trait StateTreeVisitor {
    fn visit_fungible_vault(
        &mut self,
        _vault_id: ObjectId,
        _info: &VaultInfoSubstate,
        _resource: &LiquidFungibleResource,
    ) {
    }

    fn visit_non_fungible_vault(
        &mut self,
        _vault_id: ObjectId,
        _info: &VaultInfoSubstate,
        _resource: &LiquidNonFungibleResource,
    ) {
    }

    fn visit_node_id(&mut self, _parent_id: Option<&SubstateId>, _node_id: &NodeId, _depth: u32) {}
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
        parent_node_id: Option<&SubstateId>,
        node_id: NodeId,
    ) -> Result<(), StateTreeTraverserError> {
        self.traverse_recursive(parent_node_id, node_id, 0)
    }

    fn traverse_recursive(
        &mut self,
        parent: Option<&SubstateId>,
        node_id: NodeId,
        depth: u32,
    ) -> Result<(), StateTreeTraverserError> {
        if depth > self.max_depth {
            return Err(StateTreeTraverserError::MaxDepthExceeded);
        }
        self.visitor.visit_node_id(parent, &node_id, depth);
        match node_id {
            NodeId::KeyValueStore(kv_store_id) => {
                let map = self.substate_db.get_kv_store_entries(&kv_store_id);
                for (entry_id, substate) in map.iter() {
                    let substate_id = SubstateId(
                        NodeId::KeyValueStore(kv_store_id),
                        TypedModuleId::ObjectState,
                        SubstateOffset::KeyValueStore(KeyValueStoreOffset::Entry(entry_id.clone())),
                    );
                    if let PersistedSubstate::KeyValueStoreEntry(entry) = substate {
                        if let Some(value) = entry {
                            let (_, own, _) =
                                IndexedScryptoValue::from_scrypto_value(value.clone()).unpack();
                            for child_node_id in own {
                                self.traverse_recursive(
                                    Some(&substate_id),
                                    child_node_id,
                                    depth + 1,
                                )
                                .expect("Broken Node Store");
                            }
                        }
                    }
                }
            }
            NodeId::Object(..) => {
                let substate_id = SubstateId(
                    node_id,
                    TypedModuleId::TypeInfo,
                    SubstateOffset::TypeInfo(TypeInfoOffset::TypeInfo),
                );
                let output_value = self
                    .substate_db
                    .get_substate(&substate_id)
                    .expect("Broken Node Store");
                let runtime_substate = output_value.substate.to_runtime();
                let type_substate: TypeInfoSubstate = runtime_substate.into();

                match type_substate {
                    TypeInfoSubstate::Object {
                        package_address,
                        blueprint_name,
                        ..
                    } if package_address.eq(&RESOURCE_MANAGER_PACKAGE)
                        && blueprint_name.eq(VAULT_BLUEPRINT) =>
                    {
                        if let Some(output_value) = self.substate_db.get_substate(&SubstateId(
                            node_id,
                            TypedModuleId::ObjectState,
                            SubstateOffset::Vault(VaultOffset::Info),
                        )) {
                            let info: VaultInfoSubstate = output_value.substate.into();
                            match &info.resource_type {
                                ResourceType::Fungible { .. } => {
                                    let liquid: LiquidFungibleResource = self
                                        .substate_db
                                        .get_substate(&SubstateId(
                                            node_id,
                                            TypedModuleId::ObjectState,
                                            SubstateOffset::Vault(VaultOffset::LiquidFungible),
                                        ))
                                        .unwrap()
                                        .substate
                                        .into();

                                    self.visitor.visit_fungible_vault(
                                        node_id.into(),
                                        &info,
                                        &liquid,
                                    );
                                }
                                ResourceType::NonFungible { .. } => {
                                    let liquid: LiquidNonFungibleResource = self
                                        .substate_db
                                        .get_substate(&SubstateId(
                                            node_id,
                                            TypedModuleId::ObjectState,
                                            SubstateOffset::Vault(VaultOffset::LiquidNonFungible),
                                        ))
                                        .unwrap()
                                        .substate
                                        .into();

                                    self.visitor.visit_non_fungible_vault(
                                        node_id.into(),
                                        &info,
                                        &liquid,
                                    );
                                }
                            }
                        } else {
                            return Err(StateTreeTraverserError::RENodeNotFound(node_id));
                        }
                    }
                    _ => {
                        let substate_id = SubstateId(
                            node_id,
                            TypedModuleId::ObjectState,
                            SubstateOffset::Component(ComponentOffset::State0),
                        );
                        let output_value = self
                            .substate_db
                            .get_substate(&substate_id)
                            .expect("Broken Node Store");
                        let runtime_substate = output_value.substate.to_runtime();
                        let substate_ref = runtime_substate.to_ref();
                        let (_, owned_nodes) = substate_ref.references_and_owned_nodes();
                        for child_node_id in owned_nodes {
                            self.traverse_recursive(Some(&substate_id), child_node_id, depth + 1)
                                .expect("Broken Node Store");
                        }
                    }
                }
            }
            NodeId::GlobalObject(Address::Component(ComponentAddress::Account(..)))
            | NodeId::GlobalObject(Address::Component(
                ComponentAddress::EcdsaSecp256k1VirtualAccount(..),
            ))
            | NodeId::GlobalObject(Address::Component(
                ComponentAddress::EddsaEd25519VirtualAccount(..),
            )) => {
                let substate_id = SubstateId(
                    node_id,
                    TypedModuleId::ObjectState,
                    SubstateOffset::Account(AccountOffset::Account),
                );
                let output_value = self
                    .substate_db
                    .get_substate(&substate_id)
                    .expect("Broken Node Store");
                let runtime_substate = output_value.substate.to_runtime();
                let substate_ref = runtime_substate.to_ref();
                let (_, owned_nodes) = substate_ref.references_and_owned_nodes();
                for child_node_id in owned_nodes {
                    self.traverse_recursive(Some(&substate_id), child_node_id, depth + 1)
                        .expect("Broken Node Store");
                }
            }
            NodeId::GlobalObject(Address::Component(_)) => {
                let substate_id = SubstateId(
                    node_id,
                    TypedModuleId::ObjectState,
                    SubstateOffset::Component(ComponentOffset::State0),
                );
                let output_value = self
                    .substate_db
                    .get_substate(&substate_id)
                    .expect("Broken Node Store");
                let runtime_substate = output_value.substate.to_runtime();
                let substate_ref = runtime_substate.to_ref();
                let (_, owned_nodes) = substate_ref.references_and_owned_nodes();
                for child_node_id in owned_nodes {
                    self.traverse_recursive(Some(&substate_id), child_node_id, depth + 1)
                        .expect("Broken Node Store");
                }
            }
            _ => {}
        };

        Ok(())
    }
}
