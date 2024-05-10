// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

#[derive(Debug)]
pub enum DeltaError {
    PatchError(String),
    EmptyBuffer,
    EmptySourceBuffer,
    EmptyTargetBuffer,
    InvalidDelta,
    InvalidSourceSize,
}
