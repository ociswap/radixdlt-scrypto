use scrypto::prelude::*;

#[blueprint]
mod cross_package {
    const PACKAGE_ADDRESS_PLACE_HOLDER: PackageAddress = PackageAddress::new_or_panic([
        13, 144, 99, 24, 198, 49, 140, 100, 247, 152, 202, 204, 99, 24, 198, 49, 140, 247, 189,
        241, 172, 105, 67, 234, 38, 49, 140, 99, 24, 198,
    ]);

    struct Sample {}

    impl Sample {
        pub fn call_external_package() {
            let _: () = Runtime::call_function(
                PACKAGE_ADDRESS_PLACE_HOLDER,
                "Sample",
                "callee",
                scrypto_args!(),
            );
        }

        pub fn callee() {}
    }
}