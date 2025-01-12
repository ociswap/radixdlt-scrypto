use radix_engine_interface::api::field_api::LockFlags;
use radix_engine_interface::api::ACTOR_STATE_SELF;
use radix_engine_interface::data::scrypto::{
    scrypto_decode, scrypto_encode, ScryptoDecode, ScryptoEncode, ScryptoValue,
};
use radix_engine_interface::types::*;
use sbor::rust::fmt;
use sbor::rust::marker::PhantomData;
use sbor::rust::ops::{Deref, DerefMut};
use sbor::rust::prelude::*;
use scrypto::engine::scrypto_env::ScryptoVmV1Api;

pub struct DataRef<V: ScryptoEncode> {
    lock_handle: SubstateHandle,
    value: V,
}

impl<V: fmt::Display + ScryptoEncode> fmt::Display for DataRef<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<V: ScryptoEncode> DataRef<V> {
    pub fn new(lock_handle: SubstateHandle, substate: V) -> DataRef<V> {
        DataRef {
            lock_handle,
            value: substate,
        }
    }
}

impl<V: ScryptoEncode> Deref for DataRef<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<V: ScryptoEncode> Drop for DataRef<V> {
    fn drop(&mut self) {
        ScryptoVmV1Api::field_entry_close(self.lock_handle);
    }
}

pub enum OriginalData {
    KeyValueStoreEntry(ScryptoValue),
    ComponentAppState(Vec<u8>),
}

pub struct DataRefMut<V: ScryptoEncode> {
    lock_handle: SubstateHandle,
    original_data: OriginalData,
    value: V,
}

impl<V: fmt::Display + ScryptoEncode> fmt::Display for DataRefMut<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<V: ScryptoEncode> DataRefMut<V> {
    pub fn new(
        lock_handle: SubstateHandle,
        original_data: OriginalData,
        value: V,
    ) -> DataRefMut<V> {
        DataRefMut {
            lock_handle,
            original_data,
            value,
        }
    }
}

impl<V: ScryptoEncode> Drop for DataRefMut<V> {
    fn drop(&mut self) {
        let substate = match &self.original_data {
            OriginalData::KeyValueStoreEntry(_) => scrypto_encode(&Some(&self.value)).unwrap(),
            OriginalData::ComponentAppState(_) => scrypto_encode(&self.value).unwrap(),
        };
        ScryptoVmV1Api::field_entry_write(self.lock_handle, substate);
        ScryptoVmV1Api::field_entry_close(self.lock_handle);
    }
}

impl<V: ScryptoEncode> Deref for DataRefMut<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<V: ScryptoEncode> DerefMut for DataRefMut<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

pub struct ComponentStatePointer<V: 'static + ScryptoEncode + ScryptoDecode> {
    phantom_data: PhantomData<V>,
}

impl<V: 'static + ScryptoEncode + ScryptoDecode> ComponentStatePointer<V> {
    pub fn new() -> Self {
        Self {
            phantom_data: PhantomData,
        }
    }

    pub fn get(&self) -> DataRef<V> {
        let lock_handle =
            ScryptoVmV1Api::actor_open_field(ACTOR_STATE_SELF, 0u8, LockFlags::read_only());
        let raw_substate = ScryptoVmV1Api::field_entry_read(lock_handle);
        let value: V = scrypto_decode(&raw_substate).unwrap();
        DataRef { lock_handle, value }
    }

    pub fn get_mut(&mut self) -> DataRefMut<V> {
        let lock_handle =
            ScryptoVmV1Api::actor_open_field(ACTOR_STATE_SELF, 0u8, LockFlags::MUTABLE);
        let raw_substate = ScryptoVmV1Api::field_entry_read(lock_handle);
        let value: V = scrypto_decode(&raw_substate).unwrap();
        DataRefMut {
            lock_handle,
            original_data: OriginalData::ComponentAppState(raw_substate),
            value,
        }
    }
}
