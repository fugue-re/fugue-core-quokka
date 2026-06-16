use std::collections::BTreeMap;

use fugue_core::ir::traits::{
    AsCodeBlockRef, AsFunctionRef, CodeBlockTable as _, FunctionTable as _,
};
use fugue_core::ir::{Address, CodeBlock, CodeBlockId, Function};
use fugue_core::project::Project;
use fugue_core::storage::ProjectStorageProvider;

use crate::error::QuokkaBuilderError;
use crate::export::segments::{MissingAddressPolicy, SegmentIndex};
use crate::proto::{self, quokka};

pub(crate) struct FunctionExporter;

impl FunctionExporter {
    pub(crate) fn populate<S>(
        project: &Project<S>,
        segments: &SegmentIndex,
        quokka: &mut proto::Quokka,
    ) -> Result<(), QuokkaBuilderError>
    where
        S: ProjectStorageProvider,
    {
        let mut functions = project.functions().iter().collect::<Vec<_>>();
        functions.sort_by_key(|function| function.as_ref().entry());

        for function in functions {
            quokka
                .functions
                .push(Self::convert_function(project, segments, function)?);
        }

        Ok(())
    }

    fn convert_function<'a, S, F>(
        project: &'a Project<S>,
        segments: &SegmentIndex,
        function: F,
    ) -> Result<quokka::Function, QuokkaBuilderError>
    where
        S: ProjectStorageProvider,
        F: AsFunctionRef<'a>,
    {
        let function = function.as_ref();
        let location = segments.resolve(project, function.entry(), MissingAddressPolicy::Zero)?;
        let mut blocks = Self::function_blocks(project, function)?;
        blocks.sort_by_key(|(_, block)| segments.sort_key(block.as_ref().start()));

        let block_indexes = blocks
            .iter()
            .enumerate()
            .map(|(index, (_, block))| (block.as_ref().id(), index))
            .collect::<BTreeMap<_, _>>();

        let mut converted_blocks = Vec::with_capacity(blocks.len());
        for (_, block) in &blocks {
            converted_blocks.push(Self::convert_block(
                project,
                segments,
                function,
                block.as_ref(),
            )?);
        }

        Ok(quokka::Function {
            segment_index: location.segment_index(),
            segment_offset: location.segment_offset(),
            file_offset: location.file_offset(),
            blocks: converted_blocks,
            edges: Self::convert_edges(&blocks, &block_indexes)?,
            function_type: Self::function_type(function) as i32,
            name: Self::function_name(function),
            block_positions: Vec::new(),
            mangled_name: String::new(),
            calling_convention: None,
            decompiled_code: String::new(),
            prototype: String::new(),
            comments: Vec::new(),
            edits: None,
            is_exported: false,
        })
    }

    fn function_blocks<'a, S>(
        project: &'a Project<S>,
        function: &Function,
    ) -> Result<
        Vec<(
            Address,
            <<S::ProjectStorage as fugue_core::storage::ProjectStorage>::CodeBlockTable as fugue_core::ir::traits::CodeBlockTable>::CodeBlockRef<'a>,
        )>,
        QuokkaBuilderError,
    >
    where
        S: ProjectStorageProvider,
    {
        let mut blocks = Vec::new();

        for (address, block_id) in function.blocks() {
            let Some(block) = project.blocks().get_by_id(block_id) else {
                return Err(QuokkaBuilderError::MissingBlock {
                    function: function.entry(),
                    block: block_id,
                });
            };
            blocks.push((address, block));
        }

        Ok(blocks)
    }

    fn convert_block<S>(
        project: &Project<S>,
        segments: &SegmentIndex,
        function: &Function,
        block: &CodeBlock,
    ) -> Result<quokka::Block, QuokkaBuilderError>
    where
        S: ProjectStorageProvider,
    {
        let location = segments.resolve(project, block.start(), MissingAddressPolicy::Zero)?;

        Ok(quokka::Block {
            segment_index: location.segment_index(),
            segment_offset: location.segment_offset(),
            file_offset: location.file_offset(),
            block_type: Self::block_type(function, block, location.file_offset()) as i32,
            instruction_index: Vec::new(),
            instructions_xref_to: Vec::new(),
            instructions_xref_from: Vec::new(),
            size: u32::try_from(block.len())
                .map_err(|source| QuokkaBuilderError::numeric("block size", source))?,
            is_thumb: false,
            n_instr: u32::try_from(block.instructions().len())
                .map_err(|source| QuokkaBuilderError::numeric("instruction count", source))?,
            instruction_comments: Vec::new(),
        })
    }

    fn convert_edges<'a, B>(
        blocks: &[(Address, B)],
        block_indexes: &BTreeMap<CodeBlockId, usize>,
    ) -> Result<Vec<quokka::function::Edge>, QuokkaBuilderError>
    where
        B: AsCodeBlockRef<'a>,
    {
        let mut edges = Vec::new();

        for (source_index, (_, block)) in blocks.iter().enumerate() {
            let destinations = block
                .as_ref()
                .successors()
                .iter()
                .filter_map(|successor| block_indexes.get(&successor).copied())
                .collect::<Vec<_>>();
            let edge_type = Self::edge_type(destinations.len()) as i32;
            let source = u32::try_from(source_index)
                .map_err(|source| QuokkaBuilderError::numeric("edge source", source))?;

            for destination_index in destinations {
                edges.push(quokka::function::Edge {
                    edge_type,
                    source,
                    destination: u32::try_from(destination_index).map_err(|source| {
                        QuokkaBuilderError::numeric("edge destination", source)
                    })?,
                    user_defined: false,
                });
            }
        }

        Ok(edges)
    }

    fn function_type(function: &Function) -> quokka::function::FunctionType {
        if function.is_thunk() {
            quokka::function::FunctionType::TypeThunk
        } else if function.is_external() {
            quokka::function::FunctionType::TypeImported
        } else {
            quokka::function::FunctionType::TypeNormal
        }
    }

    fn function_name(function: &Function) -> String {
        function
            .name()
            .map(|name| name.as_str().to_owned())
            .unwrap_or_else(|| format!("sub_{:x}", function.entry().offset()))
    }

    fn block_type(
        function: &Function,
        block: &CodeBlock,
        file_offset: i64,
    ) -> quokka::block::BlockType {
        if function.is_external() || file_offset < 0 {
            quokka::block::BlockType::Extern
        } else if block.has_unresolved() {
            quokka::block::BlockType::Indjump
        } else if block.is_non_returning() {
            quokka::block::BlockType::Noret
        } else if block.is_exit() {
            quokka::block::BlockType::Ret
        } else {
            quokka::block::BlockType::Normal
        }
    }

    fn edge_type(out_degree: usize) -> quokka::EdgeType {
        match out_degree {
            1 => quokka::EdgeType::EdgeJumpUncond,
            2 => quokka::EdgeType::EdgeJumpCond,
            0 => quokka::EdgeType::EdgeUnknown,
            _ => quokka::EdgeType::EdgeJumpIndir,
        }
    }
}
