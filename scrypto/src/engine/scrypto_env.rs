use crate::engine::wasm_api::*;
use radix_engine_common::math::Decimal;
use radix_engine_common::types::GlobalAddressReservation;
use radix_engine_interface::api::actor_api::EventFlags;
use radix_engine_interface::api::key_value_entry_api::KeyValueEntryHandle;
use radix_engine_interface::api::key_value_store_api::KeyValueStoreGenericArgs;
use radix_engine_interface::api::{ActorRefHandle, FieldValue};
use radix_engine_interface::api::{FieldIndex, LockFlags, ModuleId};
use radix_engine_interface::crypto::Hash;
use radix_engine_interface::data::scrypto::*;
use radix_engine_interface::types::PackageAddress;
use radix_engine_interface::types::{BlueprintId, GlobalAddress};
use radix_engine_interface::types::{Level, NodeId, SubstateHandle};
use radix_engine_interface::*;
use sbor::rust::prelude::*;

pub struct ScryptoVmV1Api;

impl ScryptoVmV1Api {
    pub fn blueprint_call(
        package_address: PackageAddress,
        blueprint_name: &str,
        function_name: &str,
        args: Vec<u8>,
    ) -> Vec<u8> {
        let blueprint_id = BlueprintId::new(&package_address, blueprint_name);
        let blueprint_id = scrypto_encode(&blueprint_id).unwrap();

        copy_buffer(unsafe {
            blueprint::blueprint_call(
                blueprint_id.as_ptr(),
                blueprint_id.len(),
                function_name.as_ptr(),
                function_name.len(),
                args.as_ptr(),
                args.len(),
            )
        })
    }

    pub fn object_new(
        blueprint_ident: &str,
        object_states: BTreeMap<FieldIndex, FieldValue>,
    ) -> NodeId {
        let object_states = scrypto_encode(&object_states).unwrap();

        let bytes = copy_buffer(unsafe {
            object::object_new(
                blueprint_ident.as_ptr(),
                blueprint_ident.len(),
                object_states.as_ptr(),
                object_states.len(),
            )
        });
        scrypto_decode(&bytes).unwrap()
    }

    pub fn object_globalize(
        object_id: NodeId,
        modules: BTreeMap<ModuleId, NodeId>,
        address_reservation: Option<GlobalAddressReservation>,
    ) -> GlobalAddress {
        let modules = scrypto_encode(&modules).unwrap();
        let address_reservation = scrypto_encode(&address_reservation).unwrap();

        let bytes = copy_buffer(unsafe {
            object::object_globalize(
                object_id.as_ref().as_ptr(),
                object_id.as_ref().len(),
                modules.as_ptr(),
                modules.len(),
                address_reservation.as_ptr(),
                address_reservation.len(),
            )
        });
        scrypto_decode(&bytes).unwrap()
    }

    pub fn object_get_blueprint_id(node_id: &NodeId) -> BlueprintId {
        let bytes = copy_buffer(unsafe {
            object::object_get_blueprint_id(node_id.as_ref().as_ptr(), node_id.as_ref().len())
        });

        scrypto_decode(&bytes).unwrap()
    }

    pub fn object_get_outer_object(node_id: &NodeId) -> GlobalAddress {
        let bytes = copy_buffer(unsafe {
            object::object_get_outer_object(node_id.as_ref().as_ptr(), node_id.as_ref().len())
        });

        scrypto_decode(&bytes).unwrap()
    }

    pub fn object_call(receiver: &NodeId, method_name: &str, args: Vec<u8>) -> Vec<u8> {
        copy_buffer(unsafe {
            object::object_call(
                receiver.as_ref().as_ptr(),
                receiver.as_ref().len(),
                method_name.as_ptr(),
                method_name.len(),
                args.as_ptr(),
                args.len(),
            )
        })
    }

    pub fn object_call_module(
        receiver: &NodeId,
        module_id: ModuleId,
        method_name: &str,
        args: Vec<u8>,
    ) -> Vec<u8> {
        copy_buffer(unsafe {
            object::object_call_module(
                receiver.as_ref().as_ptr(),
                receiver.as_ref().len(),
                module_id as u8 as u32,
                method_name.as_ptr(),
                method_name.len(),
                args.as_ptr(),
                args.len(),
            )
        })
    }

    pub fn object_call_direct(receiver: &NodeId, method_name: &str, args: Vec<u8>) -> Vec<u8> {
        copy_buffer(unsafe {
            object::object_call_direct(
                receiver.as_ref().as_ptr(),
                receiver.as_ref().len(),
                method_name.as_ptr(),
                method_name.len(),
                args.as_ptr(),
                args.len(),
            )
        })
    }

    pub fn kv_store_new(schema: KeyValueStoreGenericArgs) -> NodeId {
        let schema = scrypto_encode(&schema).unwrap();
        let bytes = copy_buffer(unsafe { kv_store::kv_store_new(schema.as_ptr(), schema.len()) });
        scrypto_decode(&bytes).unwrap()
    }

    pub fn kv_store_open_entry(
        node_id: &NodeId,
        key: &Vec<u8>,
        flags: LockFlags,
    ) -> KeyValueEntryHandle {
        let handle = unsafe {
            kv_store::kv_store_open_entry(
                node_id.as_ref().as_ptr(),
                node_id.as_ref().len(),
                key.as_ptr(),
                key.len(),
                flags.bits(),
            )
        };

        handle
    }

    pub fn kv_store_remove_entry(node_id: &NodeId, key: &Vec<u8>) -> Vec<u8> {
        let removed = copy_buffer(unsafe {
            kv_store::kv_store_remove_entry(
                node_id.as_ref().as_ptr(),
                node_id.as_ref().len(),
                key.as_ptr(),
                key.len(),
            )
        });
        removed
    }

    pub fn actor_open_field(object_handle: u32, field: u8, flags: LockFlags) -> SubstateHandle {
        let handle =
            unsafe { actor::actor_open_field(object_handle, u32::from(field), flags.bits()) };
        handle
    }

    pub fn actor_get_object_id(actor_ref_handle: ActorRefHandle) -> NodeId {
        let node_id = copy_buffer(unsafe { actor::actor_get_object_id(actor_ref_handle) });

        scrypto_decode(&node_id).unwrap()
    }

    pub fn actor_get_blueprint_id() -> BlueprintId {
        let blueprint_id = copy_buffer(unsafe { actor::actor_get_blueprint_id() });

        scrypto_decode(&blueprint_id).unwrap()
    }

    pub fn actor_emit_event(event_name: String, event_data: Vec<u8>, flags: EventFlags) {
        unsafe {
            actor::actor_emit_event(
                event_name.as_ptr(),
                event_name.len(),
                event_data.as_ptr(),
                event_data.len(),
                flags.bits(),
            )
        };
    }

    pub fn field_entry_read(lock_handle: SubstateHandle) -> Vec<u8> {
        copy_buffer(unsafe { field_entry::field_entry_read(lock_handle) })
    }

    pub fn field_entry_write(lock_handle: SubstateHandle, buffer: Vec<u8>) {
        unsafe { field_entry::field_entry_write(lock_handle, buffer.as_ptr(), buffer.len()) };
    }

    pub fn field_entry_close(lock_handle: SubstateHandle) {
        unsafe { field_entry::field_entry_close(lock_handle) };
    }

    pub fn kv_entry_read(handle: KeyValueEntryHandle) -> Vec<u8> {
        copy_buffer(unsafe { kv_entry::kv_entry_read(handle) })
    }

    pub fn kv_entry_write(handle: KeyValueEntryHandle, buffer: Vec<u8>) {
        unsafe { kv_entry::kv_entry_write(handle, buffer.as_ptr(), buffer.len()) };
    }

    pub fn kv_entry_remove(handle: KeyValueEntryHandle) -> Vec<u8> {
        copy_buffer(unsafe { kv_entry::kv_entry_remove(handle) })
    }

    pub fn kv_entry_close(handle: KeyValueEntryHandle) {
        unsafe { kv_entry::kv_entry_close(handle) };
    }

    pub fn costing_get_execution_cost_unit_limit() -> u32 {
        unsafe { costing::costing_get_execution_cost_unit_limit() }
    }

    pub fn costing_get_execution_cost_unit_price() -> Decimal {
        let bytes = copy_buffer(unsafe { costing::costing_get_execution_cost_unit_price() });
        scrypto_decode(&bytes).unwrap()
    }

    pub fn costing_get_finalization_cost_unit_limit() -> u32 {
        unsafe { costing::costing_get_finalization_cost_unit_limit() }
    }

    pub fn costing_get_finalization_cost_unit_price() -> Decimal {
        let bytes = copy_buffer(unsafe { costing::costing_get_finalization_cost_unit_price() });
        scrypto_decode(&bytes).unwrap()
    }

    pub fn costing_get_usd_price() -> Decimal {
        let bytes = copy_buffer(unsafe { costing::costing_get_usd_price() });
        scrypto_decode(&bytes).unwrap()
    }

    pub fn costing_get_tip_percentage() -> u32 {
        unsafe { costing::costing_get_tip_percentage() }
    }

    pub fn costing_get_fee_balance() -> Decimal {
        let bytes = copy_buffer(unsafe { costing::costing_get_fee_balance() });
        scrypto_decode(&bytes).unwrap()
    }

    pub fn sys_bech32_encode_address(address: GlobalAddress) -> String {
        let global_address = scrypto_encode(&address).unwrap();
        let encoded = copy_buffer(unsafe {
            system::sys_bech32_encode_address(global_address.as_ptr(), global_address.len())
        });

        scrypto_decode(&encoded).unwrap()
    }

    pub fn sys_log(level: Level, message: String) {
        let level = scrypto_encode(&level).unwrap();
        unsafe { system::sys_log(level.as_ptr(), level.len(), message.as_ptr(), message.len()) }
    }

    pub fn sys_get_transaction_hash() -> Hash {
        let actor = copy_buffer(unsafe { system::sys_get_transaction_hash() });

        scrypto_decode(&actor).unwrap()
    }

    pub fn sys_generate_ruid() -> [u8; 32] {
        let actor = copy_buffer(unsafe { system::sys_generate_ruid() });

        scrypto_decode(&actor).unwrap()
    }

    pub fn sys_panic(message: String) {
        unsafe {
            system::sys_panic(message.as_ptr(), message.len());
        };
    }
}
