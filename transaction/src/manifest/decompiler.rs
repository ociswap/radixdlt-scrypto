use radix_engine_interface::address::{AddressError, Bech32Encoder};
use radix_engine_interface::api::types::{BucketId, GlobalAddress, ProofId};
use radix_engine_interface::core::NetworkDefinition;
use radix_engine_interface::data::{
    scrypto_encode, IndexedScryptoValue, ScryptoEncode, ScryptoValueDecodeError,
    ValueFormattingContext,
};
use sbor::rust::collections::*;
use sbor::rust::fmt;
use sbor::{EncodeError, SborValue};
use utils::ContextualDisplay;

use crate::errors::*;
use crate::model::*;
use crate::validation::*;

#[derive(Debug, Clone)]
pub enum DecompileError {
    InvalidAddress(AddressError),
    InvalidArguments,
    InvalidScryptoValue(ScryptoValueDecodeError),
    InvalidSborValue(EncodeError),
    IdAllocationError(IdAllocationError),
    FormattingError(fmt::Error),
}

impl From<ScryptoValueDecodeError> for DecompileError {
    fn from(error: ScryptoValueDecodeError) -> Self {
        Self::InvalidScryptoValue(error)
    }
}

impl From<EncodeError> for DecompileError {
    fn from(error: EncodeError) -> Self {
        Self::InvalidSborValue(error)
    }
}

impl From<fmt::Error> for DecompileError {
    fn from(error: fmt::Error) -> Self {
        Self::FormattingError(error)
    }
}

pub struct DecompilationContext<'a> {
    pub bech32_encoder: Option<&'a Bech32Encoder>,
    pub id_allocator: IdAllocator,
    pub bucket_names: HashMap<BucketId, String>,
    pub proof_names: HashMap<ProofId, String>,
}

impl<'a> DecompilationContext<'a> {
    pub fn new(bech32_encoder: &'a Bech32Encoder) -> Self {
        Self {
            bech32_encoder: Some(bech32_encoder),
            id_allocator: IdAllocator::new(IdSpace::Transaction),
            bucket_names: HashMap::<BucketId, String>::new(),
            proof_names: HashMap::<ProofId, String>::new(),
        }
    }

    pub fn new_with_optional_network(bech32_encoder: Option<&'a Bech32Encoder>) -> Self {
        Self {
            bech32_encoder,
            id_allocator: IdAllocator::new(IdSpace::Transaction),
            bucket_names: HashMap::<BucketId, String>::new(),
            proof_names: HashMap::<ProofId, String>::new(),
        }
    }

    pub fn for_value_display(&'a self) -> ValueFormattingContext<'a> {
        ValueFormattingContext::with_manifest_context(
            self.bech32_encoder,
            &self.bucket_names,
            &self.proof_names,
        )
    }
}

/// Contract: if the instructions are from a validated notarized transaction, no error
/// should be returned.
pub fn decompile(
    instructions: &[Instruction],
    network: &NetworkDefinition,
) -> Result<String, DecompileError> {
    let bech32_encoder = Bech32Encoder::new(network);
    let mut buf = String::new();
    let mut context = DecompilationContext::new(&bech32_encoder);
    for inst in instructions {
        decompile_instruction(&mut buf, inst, &mut context)?;
        buf.push('\n');
    }

    Ok(buf)
}

pub fn decompile_instruction<F: fmt::Write>(
    f: &mut F,
    instruction: &Instruction,
    context: &mut DecompilationContext,
) -> Result<(), DecompileError> {
    match instruction {
        Instruction::TakeFromWorktop { resource_address } => {
            let bucket_id = context
                .id_allocator
                .new_bucket_id()
                .map_err(DecompileError::IdAllocationError)?;
            let name = format!("bucket{}", context.bucket_names.len() + 1);
            write!(
                f,
                "TAKE_FROM_WORKTOP ResourceAddress(\"{}\") Bucket(\"{}\");",
                resource_address.display(context.bech32_encoder),
                name
            )?;
            context.bucket_names.insert(bucket_id, name);
        }
        Instruction::TakeFromWorktopByAmount {
            amount,
            resource_address,
        } => {
            let bucket_id = context
                .id_allocator
                .new_bucket_id()
                .map_err(DecompileError::IdAllocationError)?;
            let name = format!("bucket{}", context.bucket_names.len() + 1);
            context.bucket_names.insert(bucket_id, name.clone());
            write!(
                f,
                "TAKE_FROM_WORKTOP_BY_AMOUNT Decimal(\"{}\") ResourceAddress(\"{}\") Bucket(\"{}\");",
                amount,
                resource_address.display(context.bech32_encoder),
                name
            )?;
        }
        Instruction::TakeFromWorktopByIds {
            ids,
            resource_address,
        } => {
            let bucket_id = context
                .id_allocator
                .new_bucket_id()
                .map_err(DecompileError::IdAllocationError)?;
            let name = format!("bucket{}", context.bucket_names.len() + 1);
            context.bucket_names.insert(bucket_id, name.clone());
            write!(
                f,
                "TAKE_FROM_WORKTOP_BY_IDS Array<NonFungibleId>({}) ResourceAddress(\"{}\") Bucket(\"{}\");",
                ids.iter()
                    .map(|k| format!("NonFungibleId(\"{}\")", hex::encode(&k.to_vec())))
                    .collect::<Vec<String>>()
                    .join(", "),
                resource_address.display(context.bech32_encoder),
                name
            )?;
        }
        Instruction::ReturnToWorktop { bucket_id } => {
            write!(
                f,
                "RETURN_TO_WORKTOP Bucket({});",
                context
                    .bucket_names
                    .get(&bucket_id)
                    .map(|name| format!("\"{}\"", name))
                    .unwrap_or(format!("{}u32", bucket_id))
            )?;
        }
        Instruction::AssertWorktopContains { resource_address } => {
            write!(
                f,
                "ASSERT_WORKTOP_CONTAINS ResourceAddress(\"{}\");",
                resource_address.display(context.bech32_encoder)
            )?;
        }
        Instruction::AssertWorktopContainsByAmount {
            amount,
            resource_address,
        } => {
            write!(
                f,
                "ASSERT_WORKTOP_CONTAINS_BY_AMOUNT Decimal(\"{}\") ResourceAddress(\"{}\");",
                amount,
                resource_address.display(context.bech32_encoder)
            )?;
        }
        Instruction::AssertWorktopContainsByIds {
            ids,
            resource_address,
        } => {
            write!(
                f,
                "ASSERT_WORKTOP_CONTAINS_BY_IDS Array<NonFungibleId>({}) ResourceAddress(\"{}\");",
                ids.iter()
                    .map(|k| format!("NonFungibleId(\"{}\")", hex::encode(&k.to_vec())))
                    .collect::<Vec<String>>()
                    .join(", "),
                resource_address.display(context.bech32_encoder)
            )?;
        }
        Instruction::PopFromAuthZone => {
            let proof_id = context
                .id_allocator
                .new_proof_id()
                .map_err(DecompileError::IdAllocationError)?;
            let name = format!("proof{}", context.proof_names.len() + 1);
            context.proof_names.insert(proof_id, name.clone());
            write!(f, "POP_FROM_AUTH_ZONE Proof(\"{}\");", name)?;
        }
        Instruction::PushToAuthZone { proof_id } => {
            write!(
                f,
                "PUSH_TO_AUTH_ZONE Proof({});",
                context
                    .proof_names
                    .get(&proof_id)
                    .map(|name| format!("\"{}\"", name))
                    .unwrap_or(format!("{}u32", proof_id))
            )?;
        }
        Instruction::ClearAuthZone => {
            f.write_str("CLEAR_AUTH_ZONE;")?;
        }
        Instruction::CreateProofFromAuthZone { resource_address } => {
            let proof_id = context
                .id_allocator
                .new_proof_id()
                .map_err(DecompileError::IdAllocationError)?;
            let name = format!("proof{}", context.proof_names.len() + 1);
            context.proof_names.insert(proof_id, name.clone());
            write!(
                f,
                "CREATE_PROOF_FROM_AUTH_ZONE ResourceAddress(\"{}\") Proof(\"{}\");",
                resource_address.display(context.bech32_encoder),
                name
            )?;
        }
        Instruction::CreateProofFromAuthZoneByAmount {
            amount,
            resource_address,
        } => {
            let proof_id = context
                .id_allocator
                .new_proof_id()
                .map_err(DecompileError::IdAllocationError)?;
            let name = format!("proof{}", context.proof_names.len() + 1);
            context.proof_names.insert(proof_id, name.clone());
            write!(
                f,
                "CREATE_PROOF_FROM_AUTH_ZONE_BY_AMOUNT Decimal(\"{}\") ResourceAddress(\"{}\") Proof(\"{}\");",
                amount,
                resource_address.display(context.bech32_encoder),
                name
            )?;
        }
        Instruction::CreateProofFromAuthZoneByIds {
            ids,
            resource_address,
        } => {
            let proof_id = context
                .id_allocator
                .new_proof_id()
                .map_err(DecompileError::IdAllocationError)?;
            let name = format!("proof{}", context.proof_names.len() + 1);
            context.proof_names.insert(proof_id, name.clone());
            write!(
                f,
                "CREATE_PROOF_FROM_AUTH_ZONE_BY_IDS Array<NonFungibleId>({}) ResourceAddress(\"{}\") Proof(\"{}\");",ids.iter()
                .map(|k| format!("NonFungibleId(\"{}\")", hex::encode(&k.to_vec())))
                .collect::<Vec<String>>()
                .join(", "),
                resource_address.display(context.bech32_encoder),
                name
            )?;
        }
        Instruction::CreateProofFromBucket { bucket_id } => {
            let proof_id = context
                .id_allocator
                .new_proof_id()
                .map_err(DecompileError::IdAllocationError)?;
            let name = format!("proof{}", context.proof_names.len() + 1);
            context.proof_names.insert(proof_id, name.clone());
            write!(
                f,
                "CREATE_PROOF_FROM_BUCKET Bucket({}) Proof(\"{}\");",
                context
                    .bucket_names
                    .get(&bucket_id)
                    .map(|name| format!("\"{}\"", name))
                    .unwrap_or(format!("{}u32", bucket_id)),
                name
            )?;
        }
        Instruction::CloneProof { proof_id } => {
            let proof_id2 = context
                .id_allocator
                .new_proof_id()
                .map_err(DecompileError::IdAllocationError)?;
            let name = format!("proof{}", context.proof_names.len() + 1);
            context.proof_names.insert(proof_id2, name.clone());
            write!(
                f,
                "CLONE_PROOF Proof({}) Proof(\"{}\");",
                context
                    .proof_names
                    .get(&proof_id)
                    .map(|name| format!("\"{}\"", name))
                    .unwrap_or(format!("{}u32", proof_id)),
                name
            )?;
        }
        Instruction::DropProof { proof_id } => {
            write!(
                f,
                "DROP_PROOF Proof({});",
                context
                    .proof_names
                    .get(&proof_id)
                    .map(|name| format!("\"{}\"", name))
                    .unwrap_or(format!("{}u32", proof_id)),
            )?;
        }
        Instruction::DropAllProofs => {
            f.write_str("DROP_ALL_PROOFS;")?;
        }
        Instruction::CallFunction {
            package_address,
            blueprint_name,
            function_name,
            args,
        } => {
            write!(
                f,
                "CALL_FUNCTION PackageAddress(\"{}\") \"{}\" \"{}\"",
                package_address.display(context.bech32_encoder),
                blueprint_name,
                function_name,
            )?;
            format_args(f, context, args)?;
            f.write_str(";")?;
        }
        Instruction::CallMethod {
            component_address,
            method_name,
            args,
        } => {
            f.write_str(&format!(
                "CALL_METHOD ComponentAddress({}) \"{}\"",
                component_address.display(context.bech32_encoder),
                method_name
            ))?;
            format_args(f, context, args)?;
            f.write_str(";")?;
        }
        Instruction::PublishPackage {
            code,
            abi,
            royalty_config,
            metadata,
            access_rules,
        } => {
            f.write_str("PUBLISH_PACKAGE")?;
            format_typed_value(f, context, code)?;
            format_typed_value(f, context, abi)?;
            format_typed_value(f, context, royalty_config)?;
            format_typed_value(f, context, metadata)?;
            format_typed_value(f, context, access_rules)?;
            f.write_str(";")?;
        }
        Instruction::PublishPackageWithOwner {
            code,
            abi,
            owner_badge,
        } => {
            f.write_str("PUBLISH_PACKAGE_WITH_OWNER")?;
            format_typed_value(f, context, code)?;
            format_typed_value(f, context, abi)?;
            format_typed_value(f, context, owner_badge)?;
            f.write_str(";")?;
        }
        Instruction::CreateResource {
            resource_type,
            metadata,
            access_rules,
            mint_params,
        } => {
            f.write_str("CREATE_RESOURCE")?;
            format_typed_value(f, context, resource_type)?;
            format_typed_value(f, context, metadata)?;
            format_typed_value(f, context, access_rules)?;
            format_typed_value(f, context, mint_params)?;
            f.write_str(";")?;
        }
        Instruction::CreateResourceWithOwner {
            resource_type,
            metadata,
            owner_badge,
            mint_params,
        } => {
            f.write_str("CREATE_RESOURCE")?;
            format_typed_value(f, context, resource_type)?;
            format_typed_value(f, context, metadata)?;
            format_typed_value(f, context, owner_badge)?;
            format_typed_value(f, context, mint_params)?;
            f.write_str(";")?;
        }
        Instruction::BurnResource { bucket_id } => {
            write!(
                f,
                "BURN_RESOURCE Bucket({});",
                context
                    .bucket_names
                    .get(&bucket_id)
                    .map(|name| format!("\"{}\"", name))
                    .unwrap_or(format!("{}u32", bucket_id)),
            )?;
        }
        Instruction::MintFungible {
            resource_address,
            amount,
        } => {
            f.write_str("MINT_FUNGIBLE")?;
            format_typed_value(f, context, resource_address)?;
            format_typed_value(f, context, amount)?;
            f.write_str(";")?;
        }
        Instruction::SetMetadata {
            entity_address,
            metadata,
        } => {
            f.write_str("SET_METADATA")?;
            format_entity_address(f, context, entity_address)?;
            format_typed_value(f, context, metadata)?;
            f.write_str(";")?;
        }
        Instruction::SetPackageRoyaltyConfig {
            package_address,
            royalty_config,
        } => {
            f.write_str("SET_PACKAGE_ROYALTY_CONFIG")?;
            format_typed_value(f, context, package_address)?;
            format_typed_value(f, context, royalty_config)?;
            f.write_str(";")?;
        }
        Instruction::SetComponentRoyaltyConfig {
            component_address,
            royalty_config,
        } => {
            f.write_str("SET_COMPONENT_ROYALTY_CONFIG")?;
            format_typed_value(f, context, component_address)?;
            format_typed_value(f, context, royalty_config)?;
            f.write_str(";")?;
        }
        Instruction::ClaimPackageRoyalty { package_address } => {
            f.write_str("CLAIM_PACKAGE_ROYALTY")?;
            format_typed_value(f, context, package_address)?;
            f.write_str(";")?;
        }
        Instruction::ClaimComponentRoyalty { component_address } => {
            f.write_str("CLAIM_COMPONENT_ROYALTY")?;
            format_typed_value(f, context, component_address)?;
            f.write_str(";")?;
        }
    }
    Ok(())
}

pub fn format_typed_value<F: fmt::Write, T: ScryptoEncode>(
    f: &mut F,
    context: &mut DecompilationContext,
    value: &T,
) -> Result<(), DecompileError> {
    let bytes = scrypto_encode(value).map_err(DecompileError::InvalidSborValue)?;
    let value =
        IndexedScryptoValue::from_slice(&bytes).map_err(DecompileError::InvalidScryptoValue)?;
    f.write_char(' ')?;
    write!(f, "{}", &value.display(context.for_value_display()))?;
    Ok(())
}

pub fn format_entity_address<F: fmt::Write>(
    f: &mut F,
    context: &mut DecompilationContext,
    address: &GlobalAddress,
) -> Result<(), DecompileError> {
    f.write_char(' ')?;
    match address {
        GlobalAddress::Component(address) => {
            write!(
                f,
                "ComponentAddress({})",
                &address.display(context.bech32_encoder)
            )?;
        }
        GlobalAddress::Package(address) => {
            write!(
                f,
                "PackageAddress({})",
                &address.display(context.bech32_encoder)
            )?;
        }
        GlobalAddress::Resource(address) => {
            write!(
                f,
                "ResourceAddress({})",
                &address.display(context.bech32_encoder)
            )?;
        }
        GlobalAddress::System(address) => {
            write!(
                f,
                "SystemAddress({})",
                &address.display(context.bech32_encoder)
            )?;
        }
    }

    Ok(())
}

pub fn format_args<F: fmt::Write>(
    f: &mut F,
    context: &mut DecompilationContext,
    args: &Vec<u8>,
) -> Result<(), DecompileError> {
    let value =
        IndexedScryptoValue::from_slice(&args).map_err(|_| DecompileError::InvalidArguments)?;
    if let SborValue::Tuple { fields } = value.dom {
        for field in fields {
            let bytes = scrypto_encode(&field)?;
            let arg = IndexedScryptoValue::from_slice(&bytes)
                .map_err(|_| DecompileError::InvalidArguments)?;
            f.write_char(' ')?;
            write!(f, "{}", &arg.display(context.for_value_display()))?;
        }
    } else {
        return Err(DecompileError::InvalidArguments);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::*;
    use radix_engine_interface::core::NetworkDefinition;
    use radix_engine_interface::model::NonFungibleId;

    #[test]
    fn test_decompile_complex() {
        let network = NetworkDefinition::simulator();
        let manifest_str = include_str!("../../examples/complex.rtm");
        let blobs = vec![
            include_bytes!("../../examples/code.blob").to_vec(),
            include_bytes!("../../examples/abi.blob").to_vec(),
        ];
        let manifest = compile(manifest_str, &network, blobs).unwrap();

        let manifest2 = decompile(&manifest.instructions, &network).unwrap();
        assert_eq!(
            manifest2,
            r#"CALL_METHOD ComponentAddress("account_sim1q02r73u7nv47h80e30pc3q6ylsj7mgvparm3pnsm780qgsy064") "withdraw_by_amount" Decimal("5") ResourceAddress("resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag");
TAKE_FROM_WORKTOP_BY_AMOUNT Decimal("2") ResourceAddress("resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag") Bucket("bucket1");
CALL_METHOD ComponentAddress("component_sim1q2f9vmyrmeladvz0ejfttcztqv3genlsgpu9vue83mcs835hum") "buy_gumball" Bucket("bucket1");
ASSERT_WORKTOP_CONTAINS_BY_AMOUNT Decimal("3") ResourceAddress("resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag");
ASSERT_WORKTOP_CONTAINS ResourceAddress("resource_sim1qzhdk7tq68u8msj38r6v6yqa5myc64ejx3ud20zlh9gseqtux6");
TAKE_FROM_WORKTOP ResourceAddress("resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag") Bucket("bucket2");
CREATE_PROOF_FROM_BUCKET Bucket("bucket2") Proof("proof1");
CLONE_PROOF Proof("proof1") Proof("proof2");
DROP_PROOF Proof("proof1");
DROP_PROOF Proof("proof2");
CALL_METHOD ComponentAddress("account_sim1q02r73u7nv47h80e30pc3q6ylsj7mgvparm3pnsm780qgsy064") "create_proof_by_amount" Decimal("5") ResourceAddress("resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag");
POP_FROM_AUTH_ZONE Proof("proof3");
DROP_PROOF Proof("proof3");
RETURN_TO_WORKTOP Bucket("bucket2");
TAKE_FROM_WORKTOP_BY_IDS Array<NonFungibleId>(NonFungibleId("5c200721031b84c5567b126440995d3ed5aaba0565d71e1834604819ff9c17f5e9d5dd078f")) ResourceAddress("resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag") Bucket("bucket3");
CREATE_RESOURCE Enum("Fungible", 0u8) Array<Tuple>() Array<Tuple>() Enum("Some", Enum("Fungible", Decimal("1")));
CALL_METHOD ComponentAddress("account_sim1q02r73u7nv47h80e30pc3q6ylsj7mgvparm3pnsm780qgsy064") "deposit_batch" Expression("ENTIRE_WORKTOP");
DROP_ALL_PROOFS;
CALL_METHOD ComponentAddress("component_sim1q2f9vmyrmeladvz0ejfttcztqv3genlsgpu9vue83mcs835hum") "complicated_method" Decimal("1") PreciseDecimal("2");
PUBLISH_PACKAGE_WITH_OWNER Blob("36dae540b7889956f1f1d8d46ba23e5e44bf5723aef2a8e6b698686c02583618") Blob("15e8699a6d63a96f66f6feeb609549be2688b96b02119f260ae6dfd012d16a5d") NonFungibleAddress("00ed9100551d7fae91eaf413e50a3c5a59f8b96af9f1297890a8f45c200721031b84c5567b126440995d3ed5aaba0565d71e1834604819ff9c17f5e9d5dd078f");
"#
        )
    }

    #[test]
    fn test_decompile_call_function() {
        let network = NetworkDefinition::simulator();
        let manifest = compile(
            include_str!("../../examples/call_function.rtm"),
            &network,
            vec![],
        )
        .unwrap();
        let manifest2 = decompile(&manifest.instructions, &network).unwrap();
        assert_eq!(
            manifest2,
            r#"CALL_FUNCTION PackageAddress("package_sim1qy4hrp8a9apxldp5cazvxgwdj80cxad4u8cpkaqqnhlsa3lfpe") "Blueprint" "function";
CALL_NATIVE_FUNCTION "EpochManager" "create";
CALL_NATIVE_FUNCTION "ResourceManager" "create";
CALL_NATIVE_FUNCTION "Package" "publish";
CALL_NATIVE_FUNCTION "TransactionProcessor" "run";
"#
        )
    }

    #[test]
    fn test_decompile_call_method() {
        let network = NetworkDefinition::simulator();
        let manifest = compile(
            include_str!("../../examples/call_method.rtm"),
            &network,
            vec![],
        )
        .unwrap();
        let manifest2 = decompile(&manifest.instructions, &network).unwrap();
        assert_eq!(
            manifest2,
            r#"CALL_METHOD ComponentAddress("component_sim1qgvyxt5rrjhwctw7krgmgkrhv82zuamcqkq75tkkrwgs00m736") "free_xrd";
CALL_METHOD Component("000000000000000000000000000000000000000000000000000000000000000005000000") "free_xrd";
TAKE_FROM_WORKTOP ResourceAddress("resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag") Bucket("bucket1");
CREATE_PROOF_FROM_AUTH_ZONE ResourceAddress("resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag") Proof("proof1");
CALL_NATIVE_METHOD Bucket("bucket1") "get_resource_address";
CALL_NATIVE_METHOD Bucket(1u32) "get_resource_address";
CALL_NATIVE_METHOD Bucket(513u32) "get_resource_address";
CALL_NATIVE_METHOD Bucket(1u32) "get_resource_address";
CALL_NATIVE_METHOD AuthZoneStack(1u32) "drain";
CALL_NATIVE_METHOD Worktop "drain";
CALL_NATIVE_METHOD KeyValueStore("000000000000000000000000000000000000000000000000000000000000000005000000") "method";
CALL_NATIVE_METHOD NonFungibleStore("000000000000000000000000000000000000000000000000000000000000000005000000") "method";
CALL_NATIVE_METHOD Component("000000000000000000000000000000000000000000000000000000000000000005000000") "add_access_check";
CALL_NATIVE_METHOD EpochManager("000000000000000000000000000000000000000000000000000000000000000005000000") "get_transaction_hash";
CALL_NATIVE_METHOD Vault("000000000000000000000000000000000000000000000000000000000000000005000000") "get_resource_address";
CALL_NATIVE_METHOD ResourceManager("000000000000000000000000000000000000000000000000000000000000000000000005") "burn";
CALL_NATIVE_METHOD Package("000000000000000000000000000000000000000000000000000000000000000000000005") "method";
CALL_NATIVE_METHOD Global("resource_sim1qrc4s082h9trka3yrghwragylm3sdne0u668h2sy6c9sckkpn6") "method";
"#
        )
    }

    #[test]
    fn test_decompile_any_value() {
        let network = NetworkDefinition::simulator();
        let manifest_str = include_str!("../../examples/any_value.rtm");
        let blobs = vec![include_bytes!("../../examples/code.blob").to_vec()];
        let manifest = compile(manifest_str, &network, blobs).unwrap();

        let manifest2 = decompile(&manifest.instructions, &network).unwrap();
        assert_eq!(
            manifest2,
            r#"TAKE_FROM_WORKTOP ResourceAddress("resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag") Bucket("bucket1");
CREATE_PROOF_FROM_AUTH_ZONE ResourceAddress("resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag") Proof("proof1");
CALL_METHOD ComponentAddress("component_sim1q2f9vmyrmeladvz0ejfttcztqv3genlsgpu9vue83mcs835hum") "with_all_types" PackageAddress("package_sim1qyqzcexvnyg60z7lnlwauh66nhzg3m8tch2j8wc0e70qkydk8r") ComponentAddress("account_sim1q0u9gxewjxj8nhxuaschth2mgencma2hpkgwz30s9wlslthace") ResourceAddress("resource_sim1qq8cays25704xdyap2vhgmshkkfyr023uxdtk59ddd4qs8cr5v") SystemAddress("system_sim1qne8qu4seyvzfgd94p3z8rjcdl3v0nfhv84judpum2lq7x4635") Component("000000000000000000000000000000000000000000000000000000000000000005000000") KeyValueStore("000000000000000000000000000000000000000000000000000000000000000005000000") Bucket("bucket1") Proof("proof1") Vault("000000000000000000000000000000000000000000000000000000000000000005000000") Expression("ALL_WORKTOP_RESOURCES") Blob("36dae540b7889956f1f1d8d46ba23e5e44bf5723aef2a8e6b698686c02583618") NonFungibleAddress("00ed9100551d7fae91eaf413e50a3c5a59f8b96af9f1297890a8f45c200721031b84c5567b126440995d3ed5aaba0565d71e1834604819ff9c17f5e9d5dd078f") Hash("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824") EcdsaSecp256k1PublicKey("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798") EcdsaSecp256k1Signature("0079224ea514206706298d8d620f660828f7987068d6d02757e6f3cbbf4a51ab133395db69db1bc9b2726dd99e34efc252d8258dcb003ebaba42be349f50f7765e") EddsaEd25519PublicKey("4cb5abf6ad79fbf5abbccafcc269d85cd2651ed4b885b5869f241aedf0a5ba29") EddsaEd25519Signature("ce993adc51111309a041faa65cbcf1154d21ed0ecdc2d54070bc90b9deb744aa8605b3f686fa178fba21070b4a4678e54eee3486a881e0e328251cd37966de09") Decimal("1.2") PreciseDecimal("1.2") NonFungibleId(Bytes("031b84c5567b126440995d3ed5aaba0565d71e1834604819ff9c17f5e9d5dd078f"));
"#
        )
    }

    #[test]
    fn decompiled_non_fungible_ids_are_equal_to_their_pre_compilation_representation() {
        // Arrange
        let non_fungible_ids = vec![
            NonFungibleId::U32(12),
            NonFungibleId::U64(19),
            NonFungibleId::String("HelloWorld!".to_string()),
            NonFungibleId::Decimal("1234".parse().unwrap()),
            NonFungibleId::Bytes(vec![0x12, 0x19, 0x22, 0xff, 0x3]),
            NonFungibleId::UUID(1922931322),
        ];

        let manifest = non_fungible_ids
            .iter()
            .enumerate()
            .map(|(i, id)| format!("TAKE_FROM_WORKTOP_BY_IDS Array<NonFungibleId>(NonFungibleId(\"{}\")) ResourceAddress(\"resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag\") Bucket(\"bucket{}\");\n", hex::encode(&id.to_vec()), i + 1))
            .collect::<Vec<String>>()
            .join("");

        let compiled = compile(&manifest, &NetworkDefinition::simulator(), Vec::new()).unwrap();

        // Act
        let decompiled =
            decompile(&compiled.instructions, &NetworkDefinition::simulator()).unwrap();

        // Assert
        assert_eq!(manifest, decompiled)
    }
}
