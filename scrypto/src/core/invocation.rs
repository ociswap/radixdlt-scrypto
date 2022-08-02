use sbor::rust::string::ToString;
use sbor::*;

use crate::engine::types::{BucketId, PackageAddress, ProofId, RENodeId, VaultId};
use crate::prelude::{ComponentAddress, ResourceAddress};

#[derive(Debug, Clone, TypeId, Encode, Decode)]
pub enum TypeName {
    Package,
    ResourceManager,
    TransactionProcessor,
    Blueprint(PackageAddress, String),
}

#[derive(Debug, Clone, TypeId, Encode, Decode)]
pub enum Receiver {
    Consumed(RENodeId),
    Component(ComponentAddress),
    ComponentMetaRef(ComponentAddress),
    ResourceManagerRef(ResourceAddress),
    BucketRef(BucketId),
    ProofRef(ProofId),
    VaultRef(VaultId),
    SystemRef,
    WorktopRef,
    AuthZoneRef,
}
