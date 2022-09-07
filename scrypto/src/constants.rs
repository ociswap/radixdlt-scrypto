use crate::component::{ComponentAddress, PackageAddress};
use crate::resource::*;
use crate::{address, construct_address};

/// The address of the sys-faucet package.
pub const SYS_FAUCET_PACKAGE: PackageAddress = construct_address!(
    EntityType::Package,
    72,
    223,
    194,
    44,
    177,
    98,
    231,
    38,
    12,
    132,
    2,
    197,
    57,
    40,
    72,
    34,
    129,
    17,
    124,
    16,
    161,
    221,
    137,
    22,
    103,
    240
);
/// The address of the sys-utils package.
pub const SYS_UTILS_PACKAGE: PackageAddress = construct_address!(
    EntityType::Package,
    0,
    44,
    100,
    204,
    153,
    17,
    167,
    139,
    223,
    159,
    221,
    222,
    95,
    90,
    157,
    196,
    136,
    236,
    235,
    197,
    213,
    35,
    187,
    15,
    207,
    158
);

/// The address of the account package.
pub const ACCOUNT_PACKAGE: PackageAddress = construct_address!(
    EntityType::Package,
    117,
    149,
    161,
    192,
    155,
    192,
    68,
    56,
    79,
    186,
    128,
    155,
    199,
    188,
    92,
    59,
    83,
    241,
    146,
    178,
    126,
    213,
    55,
    167,
    164,
    201
);

/// The address of the SysFaucet component
pub const SYS_FAUCET_COMPONENT: ComponentAddress = address!(EntityType::SystemComponent, 1u8);
// TODO Add other system components

/// The system token which allows access to system resources (e.g. setting epoch)
pub const SYSTEM_TOKEN: ResourceAddress = construct_address!(
    EntityType::Resource,
    185,
    23,
    55,
    238,
    138,
    77,
    229,
    157,
    73,
    218,
    212,
    13,
    229,
    86,
    14,
    87,
    84,
    70,
    106,
    200,
    76,
    245,
    67,
    46,
    169,
    93
);

/// The ECDSA virtual resource address.
pub const ECDSA_TOKEN: ResourceAddress = construct_address!(
    EntityType::Resource,
    43,
    113,
    132,
    253,
    47,
    66,
    111,
    180,
    52,
    199,
    68,
    195,
    33,
    205,
    145,
    223,
    131,
    117,
    181,
    225,
    240,
    27,
    116,
    0,
    157,
    255
);

/// The ED25519 virtual resource address.
pub const ED25519_TOKEN: ResourceAddress = address!(EntityType::Resource, 3u8);

/// The XRD resource address.
pub const RADIX_TOKEN: ResourceAddress = address!(EntityType::Resource, 4u8);
