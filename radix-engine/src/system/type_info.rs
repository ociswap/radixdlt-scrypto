use crate::errors::*;
use crate::kernel::kernel_api::KernelSubstateApi;
use crate::types::*;
use radix_engine_interface::api::field_api::LockFlags;

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub enum TypeInfoSubstate {
    Object(ObjectInfo),
    KeyValueStore(KeyValueStoreInfo),
    /// Represents the ownership of an allocated global address.
    GlobalAddressReservation(GlobalAddress),
    /// Represents a phantom global object, to make allocated global address usable.
    GlobalAddressPhantom(GlobalAddressPhantom),
}

impl TypeInfoSubstate {
    pub fn outer_object(&self) -> Option<GlobalAddress> {
        match self {
            TypeInfoSubstate::Object(ObjectInfo {
                blueprint_info:
                    BlueprintInfo {
                        outer_obj_info: OuterObjectInfo::Some { outer_object },
                        ..
                    },
                ..
            }) => Some(outer_object.clone()),
            _ => None,
        }
    }

    pub fn blueprint_id(&self) -> Option<&BlueprintId> {
        match self {
            TypeInfoSubstate::Object(ObjectInfo { blueprint_info, .. }) => {
                Some(&blueprint_info.blueprint_id)
            }
            _ => None,
        }
    }
}

pub struct TypeInfoBlueprint;

impl TypeInfoBlueprint {
    pub(crate) fn get_type<Y, L: Default>(
        receiver: &NodeId,
        api: &mut Y,
    ) -> Result<TypeInfoSubstate, RuntimeError>
    where
        Y: KernelSubstateApi<L>,
    {
        let handle = api.kernel_open_substate(
            receiver,
            TYPE_INFO_FIELD_PARTITION,
            &TypeInfoField::TypeInfo.into(),
            LockFlags::read_only(),
            L::default(),
        )?;
        let info: TypeInfoSubstate = api.kernel_read_substate(handle)?.as_typed().unwrap();
        api.kernel_close_substate(handle)?;
        Ok(info)
    }
}
