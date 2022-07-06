use sbor::rust::fmt;
use sbor::rust::str::FromStr;
use sbor::rust::string::String;
use sbor::rust::vec::Vec;
use sbor::*;
use scrypto::engine::types::StoredValueId;

use crate::abi::*;
use crate::address::Bech32Addressable;
use crate::address::ParseAddressError;
use crate::buffer::scrypto_encode;
use crate::component::*;
use crate::core::*;
use crate::engine::types::ValueId;
use crate::engine::{api::*, call_engine};
use crate::misc::*;
use crate::resource::AccessRules;

#[derive(Debug, TypeId, Encode, Decode)]
pub struct ComponentAddAccessCheckInput {
    pub access_rules: AccessRules,
}

#[derive(Debug, TypeId, Encode, Decode)]
pub struct ComponentGlobalizeInput {}

/// Represents the state of a component.
pub trait ComponentState: Encode + Decode {
    /// Instantiates a component from this data structure.
    fn instantiate(self) -> Component;
}

/// Represents an instantiated component.
#[derive(PartialEq, Eq, Hash)]
pub struct Component(pub(crate) ComponentAddress);

impl Component {
    /// Invokes a method on this component.
    pub fn call<T: Decode>(&self, method: &str, args: Vec<Vec<u8>>) -> T {
        Runtime::call_method(self.0, method, args)
    }

    /// Returns the package ID of this component.
    pub fn package_address(&self) -> PackageAddress {
        let address = DataAddress::Component(self.0, ComponentOffset::Info);
        let input = RadixEngineInput::ReadData(address);
        let output: (PackageAddress, String) = call_engine(input);
        output.0
    }

    /// Returns the blueprint name of this component.
    pub fn blueprint_name(&self) -> String {
        let address = DataAddress::Component(self.0, ComponentOffset::Info);
        let input = RadixEngineInput::ReadData(address);
        let output: (PackageAddress, String) = call_engine(input);
        output.1
    }

    pub fn add_access_check(&mut self, access_rules: AccessRules) -> &mut Self {
        let input = RadixEngineInput::InvokeSNode(
            SNodeRef::Component(self.0),
            "add_access_check".to_string(),
            scrypto_encode(&ComponentAddAccessCheckInput { access_rules }),
        );
        let _: () = call_engine(input);

        self
    }

    pub fn globalize(self) -> ComponentAddress {
        let input = RadixEngineInput::InvokeSNode(
            SNodeRef::Consumed(ValueId::Stored(StoredValueId::Component(self.0))),
            "globalize".to_string(),
            scrypto_encode(&ComponentGlobalizeInput {}),
        );
        let _: () = call_engine(input);
        self.0.clone()
    }
}

//========
// binary
//========

impl TryFrom<&[u8]> for Component {
    type Error = ParseAddressError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        let component_address = ComponentAddress::try_from(slice)?;
        Ok(Self(component_address))
    }
}

impl Component {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

scrypto_type!(Component, ScryptoType::Component, Vec::new());

//======
// text
//======

impl FromStr for Component {
    type Err = ParseAddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ComponentAddress::from_str(s).map(|a| Component(a))
    }
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for Component {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}

/// An instance of a blueprint, which lives in the ledger state.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentAddress(pub [u8; 27]);

impl ComponentAddress {}

//========
// binary
//========

impl TryFrom<&[u8]> for ComponentAddress {
    type Error = ParseAddressError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        match slice.len() {
            27 => Ok(Self(copy_u8_array(slice))),
            _ => Err(ParseAddressError::InvalidLength(slice.len())),
        }
    }
}

impl ComponentAddress {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

scrypto_type!(ComponentAddress, ScryptoType::ComponentAddress, Vec::new());

//======
// text
//======

impl Bech32Addressable for ComponentAddress {
    fn data(&self) -> &[u8] {
        &self.0
    }
}

impl FromStr for ComponentAddress {
    type Err = ParseAddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_bech32_string(s, &Runtime::transaction_network())
    }
}

impl fmt::Display for ComponentAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            self.to_bech32_string(&Runtime::transaction_network())
                .unwrap()
        )
    }
}

impl fmt::Debug for ComponentAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}
