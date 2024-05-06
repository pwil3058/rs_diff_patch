// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
%{
#[derive(Debug, Default)]
pub enum AttributeData {
    Before(String),
    After(String),
    ChunkHeader(String),
    ChunkLine(String),
    #[default]
    Default,
}

pub struct Target;
}%

%attr   AttributeData
%target Target

%%

%token  BeforePath  (---.*\n)
%token  AfterPath   (+++.*\n)
%token  ChunkHeader (@@.*@@.*\n)
%token  ChunkLine   ([ -+].*\n)

%%
Specification: Preamble DiffList.
DiffList: Diff |
    DiffList Diff
    .
Diff: BeforePath AfterPath ChunkHeader ChunkLines.
ChunkLines: ChunkLine |
    ChunkLines Chunk
    .