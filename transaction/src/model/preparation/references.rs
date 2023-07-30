use crate::internal_prelude::*;
use sbor::traversal::*;

pub fn extract_references(
    encoded: &[u8],
    expected_start: ExpectedStart<ManifestCustomValueKind>,
) -> IndexSet<Reference> {
    let mut references = index_set_new();
    let mut traverser =
        ManifestTraverser::new(&encoded, MANIFEST_SBOR_V1_MAX_DEPTH, expected_start, true);
    loop {
        let event = traverser.next_event();
        match event.event {
            TraversalEvent::ContainerStart(_) => {}
            TraversalEvent::ContainerEnd(_) => {}
            TraversalEvent::TerminalValue(r) => {
                if let TerminalValueRef::Custom(c) = r {
                    match c.0 {
                        ManifestCustomValue::Address(address) => {
                            if let ManifestAddress::Static(node_id) = address {
                                references.insert(Reference(node_id));
                            }
                        }
                        ManifestCustomValue::Bucket(_)
                        | ManifestCustomValue::Proof(_)
                        | ManifestCustomValue::AddressReservation(_)
                        | ManifestCustomValue::Expression(_)
                        | ManifestCustomValue::Blob(_)
                        | ManifestCustomValue::Decimal(_)
                        | ManifestCustomValue::BalancedDecimal(_)
                        | ManifestCustomValue::PreciseDecimal(_)
                        | ManifestCustomValue::NonFungibleLocalId(_) => {}
                    }
                }
            }
            TraversalEvent::TerminalValueBatch(_) => {}
            TraversalEvent::End => {
                break;
            }
            TraversalEvent::DecodeError(e) => {
                panic!("Unexpected decoding error: {:?}", e);
            }
        }
    }
    references
}
