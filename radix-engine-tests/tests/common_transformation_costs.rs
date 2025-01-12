use radix_engine::transaction::CostingParameters;
use radix_engine::transaction::ExecutionConfig;
use radix_engine::types::*;
use radix_engine_interface::blueprints::access_controller::ACCESS_CONTROLLER_CREATE_PROOF_IDENT;
use scrypto_unit::*;
use transaction::prelude::*;

// We run tests in this file to produce common manifest transformation costs for Core Apps, such as
// - Adding a lock_fee instruction, with account protected by single signature/badge, whichever is worse
// - Adding an amount assertion, for fungible/non-fungible, whichever is worse
// - Adding a secp256k1 or ed25519 signature, whichever is worse
// - Adding a notary signature

#[test]
fn estimate_locking_fee_from_an_account_protected_by_signature() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let network = NetworkDefinition::simulator();
    let (_pk, sk, account) = test_runner.new_ed25519_virtual_account();

    let manifest1 = ManifestBuilder::new().build();
    let tx1 = create_notarized_transaction_advanced(
        &mut test_runner,
        &network,
        manifest1,
        vec![], // no sign
        &sk,    // notarize
        false,
    );
    let receipt1 = test_runner.execute_transaction(
        validate_notarized_transaction(&network, &tx1).get_executable_with_free_credit(dec!(100)),
        CostingParameters::default(),
        ExecutionConfig::for_notarized_transaction(NetworkDefinition::simulator())
            .with_cost_breakdown(true),
    );
    receipt1.expect_commit_success();
    println!("\n{:?}", receipt1);

    let manifest2 = ManifestBuilder::new().lock_fee(account, dec!(100)).build();
    let tx2 = create_notarized_transaction_advanced(
        &mut test_runner,
        &network,
        manifest2,
        vec![&sk], // sign
        &sk,       // notarize
        false,
    );
    let receipt2 = test_runner.execute_transaction(
        validate_notarized_transaction(&network, &tx2).get_executable_with_free_credit(dec!(0)),
        CostingParameters::default(),
        ExecutionConfig::for_notarized_transaction(NetworkDefinition::simulator())
            .with_cost_breakdown(true),
    );
    receipt2.expect_commit_success();
    println!("\n{:?}", receipt2);

    println!(
        "Locking fee from an account protected by signature: {} XRD",
        receipt2
            .fee_summary
            .total_cost()
            .safe_sub(receipt1.fee_summary.total_cost())
            .unwrap()
    );
}

#[test]
fn estimate_locking_fee_from_an_account_protected_by_access_controller() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let network = NetworkDefinition::simulator();
    let (_pk1, sk1, _pk2, _sk2, _pk3, _sk3, _pk4, _sk4, account, access_controller) =
        test_runner.new_ed25519_virtual_account_with_access_controller(1);

    let manifest1 = ManifestBuilder::new().build();
    let tx1 = create_notarized_transaction_advanced(
        &mut test_runner,
        &network,
        manifest1,
        vec![], // no sign
        &sk1,   // notarize
        false,
    );
    let receipt1 = test_runner.execute_transaction(
        validate_notarized_transaction(&network, &tx1).get_executable_with_free_credit(dec!(100)),
        CostingParameters::default(),
        ExecutionConfig::for_notarized_transaction(NetworkDefinition::simulator())
            .with_cost_breakdown(true),
    );
    receipt1.expect_commit_success();
    println!("\n{:?}", receipt1);

    let manifest2 = ManifestBuilder::new()
        .call_method(
            access_controller,
            ACCESS_CONTROLLER_CREATE_PROOF_IDENT,
            manifest_args!(),
        )
        .lock_fee(account, dec!(100))
        .build();
    let tx2 = create_notarized_transaction_advanced(
        &mut test_runner,
        &network,
        manifest2,
        vec![&sk1], // sign
        &sk1,       // notarize
        false,
    );
    let receipt2 = test_runner.execute_transaction(
        validate_notarized_transaction(&network, &tx2).get_executable_with_free_credit(dec!(0)),
        CostingParameters::default(),
        ExecutionConfig::for_notarized_transaction(NetworkDefinition::simulator())
            .with_cost_breakdown(true),
    );
    receipt2.expect_commit_success();
    println!("\n{:?}", receipt2);

    println!(
        "Locking fee from an account protected by an access controller (1-4): {} XRD",
        receipt2
            .fee_summary
            .total_cost()
            .safe_sub(receipt1.fee_summary.total_cost())
            .unwrap()
    );
}

#[test]
fn estimate_asserting_worktop_contains_fungible_resource() {
    const AMOUNT: usize = 200;

    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let network = NetworkDefinition::simulator();
    let (_pk, sk, account) = test_runner.new_ed25519_virtual_account();
    let resource_address = test_runner.create_fungible_resource(AMOUNT.into(), 18, account);

    let manifest1 = ManifestBuilder::new()
        .lock_fee(account, 20)
        .withdraw_from_account(account, resource_address, AMOUNT)
        .deposit_batch(account)
        .build();
    let tx1 = create_notarized_transaction_advanced(
        &mut test_runner,
        &network,
        manifest1,
        vec![&sk], // no sign
        &sk,       // notarize
        false,
    );
    let receipt1 = test_runner.execute_transaction(
        validate_notarized_transaction(&network, &tx1).get_executable_with_free_credit(dec!(0)),
        CostingParameters::default(),
        ExecutionConfig::for_notarized_transaction(NetworkDefinition::simulator())
            .with_cost_breakdown(true),
    );
    receipt1.expect_commit_success();
    println!("\n{:?}", receipt1);

    let manifest2 = ManifestBuilder::new()
        .lock_fee(account, 20)
        .withdraw_from_account(account, resource_address, AMOUNT)
        .assert_worktop_contains(resource_address, AMOUNT)
        .deposit_batch(account)
        .build();
    let tx2 = create_notarized_transaction_advanced(
        &mut test_runner,
        &network,
        manifest2,
        vec![&sk], // sign
        &sk,       // notarize
        false,
    );
    let receipt2 = test_runner.execute_transaction(
        validate_notarized_transaction(&network, &tx2).get_executable_with_free_credit(dec!(0)),
        CostingParameters::default(),
        ExecutionConfig::for_notarized_transaction(NetworkDefinition::simulator())
            .with_cost_breakdown(true),
    );
    receipt2.expect_commit_success();
    println!("\n{:?}", receipt2);

    println!(
        "Asserting worktop contains (fungible resource; asserting amount only): {} XRD",
        receipt2
            .fee_summary
            .total_cost()
            .safe_sub(receipt1.fee_summary.total_cost())
            .unwrap()
    );
}

#[test]
fn estimate_asserting_worktop_contains_non_fungible_resource() {
    const AMOUNT: usize = 200;

    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let network = NetworkDefinition::simulator();
    let (_pk, sk, account) = test_runner.new_ed25519_virtual_account();
    let resource_address = test_runner.create_non_fungible_resource_advanced(
        NonFungibleResourceRoles::default(),
        account,
        AMOUNT,
    );

    let manifest1 = ManifestBuilder::new()
        .lock_fee(account, 20)
        .withdraw_from_account(account, resource_address, AMOUNT)
        .deposit_batch(account)
        .build();
    let tx1 = create_notarized_transaction_advanced(
        &mut test_runner,
        &network,
        manifest1,
        vec![&sk], // no sign
        &sk,       // notarize
        false,
    );
    let receipt1 = test_runner.execute_transaction(
        validate_notarized_transaction(&network, &tx1).get_executable_with_free_credit(dec!(0)),
        CostingParameters::default(),
        ExecutionConfig::for_notarized_transaction(NetworkDefinition::simulator())
            .with_cost_breakdown(true),
    );
    receipt1.expect_commit_success();
    println!("\n{:?}", receipt1);

    let manifest2 = ManifestBuilder::new()
        .lock_fee(account, 20)
        .withdraw_from_account(account, resource_address, AMOUNT)
        .assert_worktop_contains(resource_address, AMOUNT)
        .deposit_batch(account)
        .build();
    let tx2 = create_notarized_transaction_advanced(
        &mut test_runner,
        &network,
        manifest2,
        vec![&sk], // sign
        &sk,       // notarize
        false,
    );
    let receipt2 = test_runner.execute_transaction(
        validate_notarized_transaction(&network, &tx2).get_executable_with_free_credit(dec!(0)),
        CostingParameters::default(),
        ExecutionConfig::for_notarized_transaction(NetworkDefinition::simulator())
            .with_cost_breakdown(true),
    );
    receipt2.expect_commit_success();
    println!("\n{:?}", receipt2);

    println!(
        "Asserting worktop contains (non-fungible resource; asserting amount only): {} XRD",
        receipt2
            .fee_summary
            .total_cost()
            .safe_sub(receipt1.fee_summary.total_cost())
            .unwrap()
    );
}

// ED25519 signature is larger than Secp256k1 due to lack of public key recovery
// Thus, we use ed25519 when estimating signer signature cost
#[test]
fn estimate_adding_signature() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let network = NetworkDefinition::simulator();
    let (_pk1, sk1, account1) = test_runner.new_ed25519_virtual_account();
    let (_pk2, sk2, account2) = test_runner.new_ed25519_virtual_account();

    // Additional signature has an impact on the size of `AuthZone` substate.
    // We're doing 10 withdraw-deposit calls, which is "larger" than most transactions.
    // But, in theory, the cost could be even higher.
    let manifest = ManifestBuilder::new()
        .lock_fee(account1, 20)
        .then(|mut builder| {
            for _ in 0..10 {
                builder = builder
                    .withdraw_from_account(account1, XRD, 1) // require auth
                    .try_deposit_entire_worktop_or_abort(account2, None); // require no auth
            }
            builder
        })
        .build();
    let tx1 = create_notarized_transaction_advanced(
        &mut test_runner,
        &network,
        manifest.clone(),
        vec![&sk1], // signed by account 1
        &sk1,       // notarize
        false,
    );
    let receipt1 = test_runner.execute_transaction(
        validate_notarized_transaction(&network, &tx1).get_executable_with_free_credit(dec!(0)),
        CostingParameters::default(),
        ExecutionConfig::for_notarized_transaction(NetworkDefinition::simulator())
            .with_cost_breakdown(true),
    );
    receipt1.expect_commit_success();
    println!("\n{:?}", receipt1);

    let tx2 = create_notarized_transaction_advanced(
        &mut test_runner,
        &network,
        manifest,
        vec![&sk1, &sk2], // signed by account 1 & 2
        &sk1,             // notarize
        false,
    );
    let receipt2 = test_runner.execute_transaction(
        validate_notarized_transaction(&network, &tx2).get_executable_with_free_credit(dec!(0)),
        CostingParameters::default(),
        ExecutionConfig::for_notarized_transaction(NetworkDefinition::simulator())
            .with_cost_breakdown(true),
    );
    receipt2.expect_commit_success();
    println!("\n{:?}", receipt2);

    println!(
        "Adding a signer signature: {} XRD",
        receipt2
            .fee_summary
            .total_cost()
            .safe_sub(receipt1.fee_summary.total_cost())
            .unwrap()
    );
}

// Different from signer signature, no public key is needed in the notary signature (stored in header instead)
// Without signature, Secp256k1 signature is larger than ED25519 signature due to recovery byte
// Thus, we use Secp256k1 when estimating notarization cost
fn estimate_notarizing(notary_is_signatory: bool) {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let network = NetworkDefinition::simulator();
    let account1 = test_runner.new_account_advanced(OwnerRole::Updatable(AccessRule::AllowAll));
    let (_pk2, sk2, account2) = test_runner.new_virtual_account();

    // Additional signature has an impact on the size of `AuthZone` substate.
    // We're doing 10 withdraw-deposit calls, which is "larger" than most transactions.
    // But, in theory, the cost could be even higher.
    let manifest = ManifestBuilder::new()
        .lock_fee(account1, 20)
        .then(|mut builder| {
            for _ in 0..10 {
                builder = builder
                    .withdraw_from_account(account1, XRD, 1) // require auth
                    .try_deposit_entire_worktop_or_abort(account2, None); // require no auth
            }
            builder
        })
        .build();

    let receipt1 = test_runner.preview_manifest(
        manifest.clone(),
        vec![], // signed by nobody
        DEFAULT_TIP_PERCENTAGE,
        PreviewFlags::default(),
    );
    receipt1.expect_commit_success();
    println!("\n{:?}", receipt1);

    let tx2 = create_notarized_transaction_advanced(
        &mut test_runner,
        &network,
        manifest,
        vec![], // signed by nobody
        &sk2,   // notarized by account 2
        notary_is_signatory,
    );
    let receipt2 = test_runner.execute_transaction(
        validate_notarized_transaction(&network, &tx2).get_executable_with_free_credit(dec!(0)),
        CostingParameters::default(),
        ExecutionConfig::for_notarized_transaction(NetworkDefinition::simulator())
            .with_cost_breakdown(true),
    );
    receipt2.expect_commit_success();
    println!("\n{:?}", receipt2);

    println!(
        "Notarizing (notary_is_signatory: {}): {} XRD",
        notary_is_signatory,
        receipt2
            .fee_summary
            .total_cost()
            .safe_sub(receipt1.fee_summary.total_cost())
            .unwrap()
    );
}

#[test]
fn estimate_notarizing_notary_is_not_signatory() {
    estimate_notarizing(false);
}

#[test]
fn estimate_notarizing_notary_is_signatory() {
    estimate_notarizing(true);
}
