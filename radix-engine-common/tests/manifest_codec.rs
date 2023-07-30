use radix_engine_common::address::test_addresses::*;
use radix_engine_common::data::manifest::model::*;
use radix_engine_common::prelude::*;

#[derive(ManifestSbor, PartialEq, Eq, Debug)]
struct TestStruct {
    a: ManifestAddress,
    d: ManifestBucket,
    e: ManifestProof,
    f: ManifestExpression,
    g: ManifestBlobRef,
    h: ManifestDecimal,
    i: ManifestBalancedDecimal,
    j: ManifestPreciseDecimal,
    k: ManifestNonFungibleLocalId,
}

#[test]
fn test_encode_and_decode() {
    let t = TestStruct {
        a: ManifestAddress::Static(FUNGIBLE_RESOURCE_NODE_ID),
        d: ManifestBucket(4),
        e: ManifestProof(5),
        f: ManifestExpression::EntireAuthZone,
        g: ManifestBlobRef([6u8; 32]),
        h: ManifestDecimal([7u8; 32]),
        i: ManifestBalancedDecimal([8u8; 32]),
        j: ManifestPreciseDecimal([9u8; 64]),
        k: ManifestNonFungibleLocalId::string("abc".to_owned()).unwrap(),
    };

    let bytes = manifest_encode(&t).unwrap();
    assert_eq!(
        bytes,
        vec![
            77, // prefix
            33, // struct
            9,  // field length
            128, 0, 93, 166, 99, 24, 198, 49, 140, 97, 245, 166, 27, 76, 99, 24, 198, 49, 140, 247,
            148, 170, 141, 41, 95, 20, 230, 49, 140, 99, 24, 198, // address
            129, 4, 0, 0, 0, // bucket
            130, 5, 0, 0, 0, // proof
            131, 1, // expression
            132, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
            6, 6, 6, 6, 6, // blob
            133, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, // decimal
            137, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
            8, 8, 8, 8, 8, // balanced decimal
            134, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
            9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
            9, 9, 9, 9, 9, 9, 9, 9, // precise decimal
            135, 0, 3, 97, 98, 99, // non-fungible local id
        ]
    );
    let decoded: TestStruct = manifest_decode(&bytes).unwrap();
    assert_eq!(decoded, t);
}
