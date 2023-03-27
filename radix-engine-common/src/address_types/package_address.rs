use crate::address::EncodeBech32AddressError;
use crate::address::{AddressDisplayContext, EntityType, NO_NETWORK};
use crate::data::manifest::ManifestCustomValueKind;
use crate::data::scrypto::*;
use crate::well_known_scrypto_custom_type;
use crate::*;
use radix_engine_constants::NODE_ID_LENGTH;
use sbor::rust::fmt;
use sbor::rust::vec::Vec;
use sbor::*;
use utils::{copy_u8_array, ContextualDisplay};

/// A collection of blueprints, compiled and published as a single unit.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackageAddress([u8; NODE_ID_LENGTH]); // private to ensure entity type check

impl PackageAddress {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl AsRef<[u8]> for PackageAddress {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl TryFrom<&[u8]> for PackageAddress {
    type Error = ParsePackageAddressError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        match slice.len() {
            NODE_ID_LENGTH => match EntityType::from_repr(slice[0])
                .ok_or(ParsePackageAddressError::InvalidEntityTypeId(slice[0]))?
            {
                EntityType::GlobalPackage => Ok(Self(copy_u8_array(&slice[1..]))),
                _ => Err(ParsePackageAddressError::InvalidEntityTypeId(slice[0])),
            },
            _ => Err(ParsePackageAddressError::InvalidLength(slice.len())),
        }
    }
}

//========
// error
//========

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsePackageAddressError {
    InvalidLength(usize),
    InvalidEntityTypeId(u8),
}

#[cfg(not(feature = "alloc"))]
impl std::error::Error for ParsePackageAddressError {}

#[cfg(not(feature = "alloc"))]
impl fmt::Display for ParsePackageAddressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

//========
// binary
//========

well_known_scrypto_custom_type!(
    PackageAddress,
    ScryptoCustomValueKind::Address,
    Type::PackageAddress,
    NODE_ID_LENGTH,
    PACKAGE_ADDRESS_ID
);

manifest_type!(
    PackageAddress,
    ManifestCustomValueKind::Address,
    NODE_ID_LENGTH
);

//========
// text
//========

impl fmt::Debug for PackageAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.display(NO_NETWORK))
    }
}

impl<'a> ContextualDisplay<AddressDisplayContext<'a>> for PackageAddress {
    type Error = EncodeBech32AddressError;

    fn contextual_format<F: fmt::Write>(
        &self,
        f: &mut F,
        context: &AddressDisplayContext<'a>,
    ) -> Result<(), Self::Error> {
        if let Some(encoder) = context.encoder {
            return encoder.encode_to_fmt(f, &self.0);
        }

        // This could be made more performant by streaming the hex into the formatter
        write!(f, "PackageAddress({})", hex::encode(&self.0))
            .map_err(|err| EncodeBech32AddressError::FormatError(err))
    }
}
