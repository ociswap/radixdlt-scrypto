use radix_engine::blueprints::consensus_manager::ValidatorError;
use radix_engine::errors::{ApplicationError, RuntimeError};
use radix_engine::transaction::TransactionReceipt;
use radix_engine::types::*;
use radix_engine_interface::blueprints::resource::FromPublicKey;
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;
use transaction::signing::secp256k1::Secp256k1PrivateKey;

fn signal_protocol_update_test<F>(as_owner: bool, name_len: usize, result_check: F)
where
    F: Fn(TransactionReceipt) -> (),
{
    // Arrange
    let initial_epoch = Epoch::of(5);
    let pub_key = Secp256k1PrivateKey::from_u64(1u64).unwrap().public_key();
    let validator_account_address = ComponentAddress::virtual_account_from_public_key(&pub_key);
    let genesis = CustomGenesis::single_validator_and_staker(
        pub_key,
        Decimal::one(),
        validator_account_address,
        initial_epoch,
        CustomGenesis::default_consensus_manager_config(),
    );
    let mut test_runner = TestRunner::builder().with_custom_genesis(genesis).build();

    // Act
    let validator_address = test_runner.get_active_validator_with_key(&pub_key);
    let mut builder = ManifestBuilder::new();
    builder.lock_fee(test_runner.faucet_component(), 500u32.into());
    if as_owner {
        builder.create_proof_from_account(validator_account_address, VALIDATOR_OWNER_BADGE);
    }
    let manifest = builder
        .signal_protocol_update_readiness(validator_address, "a".repeat(name_len).as_str())
        .register_validator(validator_address)
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&pub_key)],
    );

    // Assert
    result_check(receipt);
}

#[test]
fn can_signal_protocol_update() {
    signal_protocol_update_test(true, 32, |e| {
        e.expect_commit_success();
    })
}

#[test]
fn cannot_signal_protocol_update_if_not_owner() {
    signal_protocol_update_test(false, 32, |e| e.expect_auth_failure())
}

#[test]
fn cannot_signal_protocol_update_if_wrong_length() {
    signal_protocol_update_test(true, 33, |e| {
        e.expect_specific_failure(|e| {
            matches!(
                e,
                RuntimeError::ApplicationError(ApplicationError::ValidatorError(
                    ValidatorError::InvalidProtocolVersionNameLength { .. }
                ))
            )
        });
    })
}