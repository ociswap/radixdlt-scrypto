use crate::blueprints::resource::*;
use crate::errors::{ApplicationError, RuntimeError};
use crate::kernel::heap::{DroppedFungibleBucket, DroppedNonFungibleBucket};
use crate::types::*;
use radix_engine_interface::api::ClientApi;
use radix_engine_interface::blueprints::resource::*;

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub enum BucketError {
    ResourceError(ResourceError),
    ProofError(ProofError),
    Locked(NodeId),
    MismatchingResource,
    InvalidAmount,
    DecimalOverflow,
}

impl From<BucketError> for RuntimeError {
    fn from(bucket_error: BucketError) -> Self {
        RuntimeError::ApplicationError(ApplicationError::BucketError(bucket_error))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct BucketInfoSubstate {
    pub resource_type: ResourceType,
}

pub fn drop_fungible_bucket<Y>(
    bucket_node_id: &NodeId,
    api: &mut Y,
) -> Result<DroppedFungibleBucket, RuntimeError>
where
    Y: ClientApi<RuntimeError>,
{
    let fields = api.drop_object(bucket_node_id)?;
    let bucket: DroppedFungibleBucket = fields.into();
    if bucket.locked.is_locked() {
        return Err(RuntimeError::ApplicationError(
            ApplicationError::BucketError(BucketError::Locked(bucket_node_id.clone())),
        ));
    }

    Ok(bucket)
}

pub fn drop_non_fungible_bucket<Y>(
    bucket_node_id: &NodeId,
    api: &mut Y,
) -> Result<DroppedNonFungibleBucket, RuntimeError>
where
    Y: ClientApi<RuntimeError>,
{
    let fields = api.drop_object(bucket_node_id)?;
    let bucket: DroppedNonFungibleBucket = fields.into();
    if bucket.locked.is_locked() {
        return Err(RuntimeError::ApplicationError(
            ApplicationError::BucketError(BucketError::Locked(bucket_node_id.clone())),
        ));
    }

    Ok(bucket)
}
