use scrypto::core::{call_method, State};
use scrypto::resource::{Bucket, BucketRef, Vault};
use scrypto::{args, blueprint, info};

use crate::utils::*;

blueprint! {
    struct MoveTest {
        vaults: Vec<Vault>
    }

    impl MoveTest {

        pub fn receive_bucket(&mut self, t: Bucket) {
            info!("Received bucket: resource_def = {}, amount = {}", t.resource_def().address(), t.amount());
            self.vaults.push(Vault::with_bucket(t));
        }

        pub fn receive_bucket_ref(&self, t: BucketRef) {
            info!("Received bucket_ref: resource_def = {}, amount = {}", t.resource_def().address(), t.amount());
            t.drop();
        }

        pub fn move_bucket() {
            let (resource_def, auth) =  create_mutable("m1");
            let bucket = resource_def.mint(100, auth.borrow());
            auth.burn();

            let component = MoveTest {
                vaults: Vec::new()
            }
            .instantiate();

            call_method(component.address(), "receive_bucket", args!(bucket));
        }

        pub fn move_bucket_ref() -> Bucket {
            let (resource_def, auth) =  create_mutable("m2");
            let bucket = resource_def.mint(100, auth.borrow());
            auth.burn();

            let component = MoveTest {
                vaults: Vec::new()
            }
            .instantiate();

            call_method(component.address(), "receive_bucket_ref", args!(bucket.borrow()));

            // The package still owns the bucket
            bucket
        }
    }
}
