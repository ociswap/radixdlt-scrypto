use radix_engine_interface::data::{scrypto_decode, ScryptoCustomTypeId};
use radix_engine_interface::engine::api::{EngineApi, SysNativeInvokable};
use radix_engine_interface::engine::types::{
    ComponentId, ComponentOffset, GlobalAddress, RENodeId, ScryptoMethodIdent, ScryptoRENode,
    ScryptoReceiver, SubstateOffset,
};
use radix_engine_interface::model::*;

use sbor::rust::borrow::ToOwned;
use sbor::rust::fmt;
use sbor::rust::fmt::Debug;
use sbor::rust::str::FromStr;
use sbor::rust::string::String;
use sbor::rust::string::ToString;
use sbor::rust::vec::Vec;
use sbor::*;
use scrypto::scrypto_type;
use utils::misc::copy_u8_array;

use crate::abi::*;
use crate::core::*;
use crate::engine::scrypto_env::ScryptoEnv;
use crate::scrypto;
use radix_engine_derive::Describe;

/// Represents the state of a component.
pub trait ComponentState<C: LocalComponent>:
    Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>
{
    /// Instantiates a component from this data structure.
    fn instantiate(self) -> C;
}

pub trait LocalComponent {
    fn package_address(&self) -> PackageAddress;
    fn blueprint_name(&self) -> String;
    fn add_access_check(&mut self, access_rules: AccessRules) -> &mut Self;
    fn globalize(self) -> ComponentAddress;
}

/// Represents an instantiated component.
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Component(pub ComponentId);

// TODO: de-duplication
#[derive(Debug, Clone, PartialEq, Eq)]
#[scrypto(TypeId, Encode, Decode, Describe)]
pub struct ComponentInfoSubstate {
    pub package_address: PackageAddress,
    pub blueprint_name: String,
    pub access_rules: Vec<AccessRules>,
}

// TODO: de-duplication
#[derive(Debug, Clone, TypeId, Encode, Decode, Describe, PartialEq, Eq)]
pub struct ComponentStateSubstate {
    pub raw: Vec<u8>,
}

impl Component {
    /// Invokes a method on this component.
    pub fn call<T: Decode<ScryptoCustomTypeId>>(&self, method: &str, args: Vec<u8>) -> T {
        let mut sys_calls = ScryptoEnv;
        let rtn = sys_calls
            .sys_invoke_scrypto_method(
                ScryptoMethodIdent {
                    receiver: ScryptoReceiver::Component(self.0),
                    method_name: method.to_string(),
                },
                args,
            )
            .unwrap();
        scrypto_decode(&rtn).unwrap()
    }

    /// Returns the package ID of this component.
    pub fn package_address(&self) -> PackageAddress {
        let pointer = DataPointer::new(
            RENodeId::Component(self.0),
            SubstateOffset::Component(ComponentOffset::Info),
        );
        let state: DataRef<ComponentInfoSubstate> = pointer.get();
        state.package_address
    }

    /// Returns the blueprint name of this component.
    pub fn blueprint_name(&self) -> String {
        let pointer = DataPointer::new(
            RENodeId::Component(self.0),
            SubstateOffset::Component(ComponentOffset::Info),
        );
        let state: DataRef<ComponentInfoSubstate> = pointer.get();
        state.blueprint_name.clone()
    }

    pub fn add_access_check(&mut self, access_rules: AccessRules) -> &mut Self {
        self.sys_add_access_check(access_rules, &mut ScryptoEnv)
            .unwrap()
    }

    pub fn sys_add_access_check<Y, E: Debug + Decode<ScryptoCustomTypeId>>(
        &mut self,
        access_rules: AccessRules,
        sys_calls: &mut Y,
    ) -> Result<&mut Self, E>
    where
        Y: EngineApi<E> + SysNativeInvokable<ComponentAddAccessCheckInvocation, E>,
    {
        sys_calls.sys_invoke(ComponentAddAccessCheckInvocation {
            receiver: RENodeId::Component(self.0),
            access_rules,
        })?;

        Ok(self)
    }

    pub fn globalize(self) -> ComponentAddress {
        self.sys_globalize(&mut ScryptoEnv).unwrap()
    }

    pub fn sys_globalize<Y, E: Debug + Decode<ScryptoCustomTypeId>>(
        self,
        sys_calls: &mut Y,
    ) -> Result<ComponentAddress, E>
    where
        Y: EngineApi<E>,
    {
        let node_id: RENodeId =
            sys_calls.sys_create_node(ScryptoRENode::GlobalComponent(self.0))?;
        Ok(node_id.into())
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BorrowedGlobalComponent(pub ComponentAddress);

impl BorrowedGlobalComponent {
    /// Invokes a method on this component.
    pub fn call<T: Decode<ScryptoCustomTypeId>>(&self, method: &str, args: Vec<u8>) -> T {
        let mut syscalls = ScryptoEnv;
        let raw = syscalls
            .sys_invoke_scrypto_method(
                ScryptoMethodIdent {
                    receiver: ScryptoReceiver::Global(self.0),
                    method_name: method.to_string(),
                },
                args,
            )
            .unwrap();
        scrypto_decode(&raw).unwrap()
    }

    /// Returns the package ID of this component.
    pub fn package_address(&self) -> PackageAddress {
        let pointer = DataPointer::new(
            RENodeId::Global(GlobalAddress::Component(self.0)),
            SubstateOffset::Component(ComponentOffset::Info),
        );
        let state: DataRef<ComponentInfoSubstate> = pointer.get();
        state.package_address
    }

    /// Returns the blueprint name of this component.
    pub fn blueprint_name(&self) -> String {
        let pointer = DataPointer::new(
            RENodeId::Global(GlobalAddress::Component(self.0)),
            SubstateOffset::Component(ComponentOffset::Info),
        );
        let state: DataRef<ComponentInfoSubstate> = pointer.get();
        state.blueprint_name.clone()
    }
}

//========
// binary
//========

/// Represents an error when decoding key value store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseComponentError {
    InvalidHex(String),
    InvalidLength(usize),
}

impl TryFrom<&[u8]> for Component {
    type Error = ParseComponentError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        match slice.len() {
            36 => Ok(Self(copy_u8_array(slice))),
            _ => Err(ParseComponentError::InvalidLength(slice.len())),
        }
    }
}

impl Component {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

scrypto_type!(
    Component,
    ScryptoCustomTypeId::Component,
    Type::Component,
    36
);

//======
// text
//======

impl FromStr for Component {
    type Err = ParseComponentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|_| ParseComponentError::InvalidHex(s.to_owned()))?;
        Self::try_from(bytes.as_slice())
    }
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", hex::encode(self.to_vec()))
    }
}

impl fmt::Debug for Component {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self.0)
    }
}
