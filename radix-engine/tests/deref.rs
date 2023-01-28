use radix_engine::types::*;
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;

#[test]
fn no_visible_component_nodes_on_deref_lock() {
    // Arrange
    let mut test_runner = TestRunner::new(true);
    let package = test_runner.compile_and_publish("./tests/blueprints/deref");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .call_function(
            package,
            "Deref",
            "verify_no_visible_component_nodes_on_deref_lock",
            args!(FAUCET_COMPONENT),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn no_visible_component_nodes_after_deref_lock_drop() {
    // Arrange
    let mut test_runner = TestRunner::new(true);
    let package = test_runner.compile_and_publish("./tests/blueprints/deref");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .call_function(
            package,
            "Deref",
            "verify_no_visible_component_nodes_after_deref_lock_drop",
            args!(FAUCET_COMPONENT),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}
