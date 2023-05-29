use radix_engine::types::*;
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;

#[test]
fn test_create_and_return() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package = test_runner.compile_and_publish("./tests/blueprints/preallocated_address");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 10.into())
        .call_function(
            package,
            "AlmightyPreallocatedAddress",
            "create_and_return",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn test_create_and_call() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package = test_runner.compile_and_publish("./tests/blueprints/preallocated_address");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 10.into())
        .call_function(
            package,
            "AlmightyPreallocatedAddress",
            "create_and_call",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn test_create_and_consume_within_frame() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package = test_runner.compile_and_publish("./tests/blueprints/preallocated_address");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 10.into())
        .call_function(
            package,
            "AlmightyPreallocatedAddress",
            "create_and_consume_within_frame",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn test_create_and_consume_in_another_frame() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package = test_runner.compile_and_publish("./tests/blueprints/preallocated_address");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 10.into())
        .call_function(
            package,
            "AlmightyPreallocatedAddress",
            "create_and_consume_in_another_frame",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn test_create_and_store_in_key_value_store() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package = test_runner.compile_and_publish("./tests/blueprints/preallocated_address");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 10.into())
        .call_function(
            package,
            "AlmightyPreallocatedAddress",
            "create_and_store_in_key_value_store",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn test_create_and_store_in_metadata() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package = test_runner.compile_and_publish("./tests/blueprints/preallocated_address");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 10.into())
        .call_function(
            package,
            "AlmightyPreallocatedAddress",
            "create_and_store_in_metadata",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}
