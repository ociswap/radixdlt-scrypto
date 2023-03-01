use radix_engine_interface::math::*;
use radix_engine_interface::*;

#[derive(NonFungibleData, ScryptoSbor)]
pub struct TestStruct {
    pub a: u32,
    #[legacy_skip]
    #[sbor(skip)]
    pub b: String,
    pub c: Decimal,
}

#[derive(ScryptoSbor)]
pub enum TestEnum {
    A { named: String },
    B(u32, u8, Decimal),
    C,
}
