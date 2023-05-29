use crate::types::*;
use radix_engine_interface::blueprints::resource::AUTH_ZONE_BLUEPRINT;
use radix_engine_interface::blueprints::transaction_processor::TRANSACTION_PROCESSOR_BLUEPRINT;
use radix_engine_interface::{api::ObjectModuleId, blueprints::resource::GlobalCaller};

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct InstanceContext {
    pub outer_object: GlobalAddress,
    pub outer_blueprint: String,
}

/// No method acting here!
#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct MethodActor {
    pub global_address: Option<GlobalAddress>,
    pub node_id: NodeId,
    pub node_object_info: ObjectInfo,

    pub module_id: ObjectModuleId,
    pub module_object_info: ObjectInfo,

    pub ident: String,
    pub instance_context: Option<InstanceContext>,
    pub is_direct_access: bool,
}

impl MethodActor {
    pub fn fn_identifier(&self) -> FnIdentifier {
        FnIdentifier {
            blueprint: self.module_object_info.blueprint.clone(),
            ident: FnIdent::Application(self.ident.to_string()),
        }
    }
}

#[derive(Debug, PartialEq, Eq, ScryptoSbor)]
pub enum Actor {
    Root,
    Method(MethodActor),
    Function {
        blueprint: BlueprintId,
        ident: String,
    },
    VirtualLazyLoad {
        blueprint: BlueprintId,
        ident: u8,
    },
}

impl Actor {
    pub fn len(&self) -> usize {
        match self {
            Actor::Root => 1,
            Actor::Method(MethodActor { node_id, ident, .. }) => {
                node_id.as_ref().len() + ident.len()
            }
            Actor::Function { blueprint, ident } => {
                blueprint.package_address.as_ref().len()
                    + blueprint.blueprint_name.len()
                    + ident.len()
            }
            Actor::VirtualLazyLoad { blueprint, .. } => {
                blueprint.package_address.as_ref().len() + blueprint.blueprint_name.len() + 1
            }
        }
    }

    pub fn is_auth_zone(&self) -> bool {
        match self {
            Actor::Method(MethodActor {
                module_object_info: object_info,
                ..
            }) => {
                object_info.blueprint.package_address.eq(&RESOURCE_PACKAGE)
                    && object_info.blueprint.blueprint_name.eq(AUTH_ZONE_BLUEPRINT)
            }
            Actor::Function { .. } => false,
            Actor::VirtualLazyLoad { .. } => false,
            Actor::Root { .. } => false,
        }
    }

    pub fn is_barrier(&self) -> bool {
        match self {
            Actor::Method(MethodActor {
                module_object_info: object_info,
                ..
            }) => object_info.global,
            Actor::Function { .. } => true,
            Actor::VirtualLazyLoad { .. } => false,
            Actor::Root { .. } => false,
        }
    }

    pub fn fn_identifier(&self) -> FnIdentifier {
        match self {
            Actor::Root => panic!("Should never be called"),
            Actor::Method(method_actor) => method_actor.fn_identifier(),
            Actor::Function { blueprint, ident } => FnIdentifier {
                blueprint: blueprint.clone(),
                ident: FnIdent::Application(ident.to_string()),
            },
            Actor::VirtualLazyLoad { blueprint, ident } => FnIdentifier {
                blueprint: blueprint.clone(),
                ident: FnIdent::System(*ident),
            },
        }
    }

    pub fn is_transaction_processor(&self) -> bool {
        match self {
            Actor::Root => false,
            Actor::Method(MethodActor {
                module_object_info: ObjectInfo { blueprint, .. },
                ..
            })
            | Actor::Function { blueprint, .. }
            | Actor::VirtualLazyLoad { blueprint, .. } => blueprint.eq(&BlueprintId::new(
                &TRANSACTION_PROCESSOR_PACKAGE,
                TRANSACTION_PROCESSOR_BLUEPRINT,
            )),
        }
    }

    pub fn try_as_method(&self) -> Option<&MethodActor> {
        match self {
            Actor::Method(actor) => Some(actor),
            _ => None,
        }
    }

    pub fn as_global_caller(&self) -> Option<GlobalCaller> {
        match self {
            Actor::Method(actor) => actor.global_address.map(|address| address.into()),
            Actor::Function { blueprint, .. } => Some(blueprint.clone().into()),
            _ => None,
        }
    }

    pub fn instance_context(&self) -> Option<InstanceContext> {
        match self {
            Actor::Method(MethodActor {
                instance_context, ..
            }) => instance_context.clone(),
            _ => None,
        }
    }

    pub fn blueprint(&self) -> &BlueprintId {
        match self {
            Actor::Method(MethodActor {
                module_object_info: ObjectInfo { blueprint, .. },
                ..
            })
            | Actor::Function { blueprint, .. }
            | Actor::VirtualLazyLoad { blueprint, .. } => blueprint,
            Actor::Root => panic!("Unexpected call"), // TODO: Should we just mock this?
        }
    }

    /// Proofs which exist only on the local call frame
    /// TODO: Update abstractions such that it is based on local call frame
    pub fn get_virtual_non_extending_proofs(&self) -> BTreeSet<NonFungibleGlobalId> {
        btreeset!(NonFungibleGlobalId::package_of_direct_caller_badge(
            *self.package_address()
        ))
    }

    pub fn get_virtual_non_extending_barrier_proofs(&self) -> BTreeSet<NonFungibleGlobalId> {
        if let Some(global_caller) = self.as_global_caller() {
            btreeset!(NonFungibleGlobalId::global_caller_badge(global_caller))
        } else {
            btreeset!()
        }
    }

    pub fn package_address(&self) -> &PackageAddress {
        let blueprint = match &self {
            Actor::Method(MethodActor {
                module_object_info: ObjectInfo { blueprint, .. },
                ..
            }) => blueprint,
            Actor::Function { blueprint, .. } => blueprint,
            Actor::VirtualLazyLoad { blueprint, .. } => blueprint,
            Actor::Root => return &PACKAGE_PACKAGE, // TODO: Should we mock this with something better?
        };

        &blueprint.package_address
    }

    pub fn blueprint_name(&self) -> &str {
        match &self {
            Actor::Method(MethodActor {
                module_object_info: ObjectInfo { blueprint, .. },
                ..
            })
            | Actor::Function { blueprint, .. }
            | Actor::VirtualLazyLoad { blueprint, .. } => blueprint.blueprint_name.as_str(),
            Actor::Root => panic!("Unexpected call"), // TODO: Should we just mock this?
        }
    }

    pub fn method(
        global_address: Option<GlobalAddress>,
        method: MethodIdentifier,
        node_object_info: ObjectInfo,
        module_object_info: ObjectInfo,
        instance_context: Option<InstanceContext>,
        is_direct_access: bool,
    ) -> Self {
        Self::Method(MethodActor {
            global_address,
            node_id: method.0,
            node_object_info,
            module_id: method.1,
            ident: method.2,
            module_object_info,
            instance_context,
            is_direct_access,
        })
    }

    pub fn function(blueprint: BlueprintId, ident: String) -> Self {
        Self::Function { blueprint, ident }
    }

    pub fn virtual_lazy_load(blueprint: BlueprintId, ident: u8) -> Self {
        Self::VirtualLazyLoad { blueprint, ident }
    }
}
