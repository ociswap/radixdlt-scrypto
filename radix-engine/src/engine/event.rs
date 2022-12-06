use crate::engine::SysCallTrace;
use crate::types::*;
use transaction::model::BasicInstruction;

#[derive(Debug)]
pub enum Event<'a> {
    Runtime(RuntimeEvent<'a>),
    Tracked(TrackedEvent),
}

#[derive(Debug)]
pub enum RuntimeEvent<'a> {
    PreExecuteManifest,
    PreExecuteInstruction {
        instruction_index: usize,
        instruction: &'a BasicInstruction,
    },
    PostExecuteInstruction {
        instruction_index: usize,
        instruction: &'a BasicInstruction,
    },
    PostExecuteManifest,
}

#[derive(Debug, Clone)]
#[scrypto(TypeId, Encode, Decode)]
pub enum TrackedEvent {
    Native(NativeEvent),
    Scrypto(Vec<u8>),
}

#[derive(Debug, Clone)]
#[scrypto(TypeId, Encode, Decode)]
pub enum NativeEvent {
    SysCallTrace(SysCallTrace),
}
