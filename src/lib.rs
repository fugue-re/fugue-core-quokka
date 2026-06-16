mod builder;
mod error;
mod export;
mod options;
pub mod proto;

pub use builder::QuokkaBuilder;
pub use error::QuokkaBuilderError;
pub use options::{DEFAULT_QUOKKA_VERSION, ExportOptions};

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Read;

    use fugue_core::ir::traits::{CodeBlockTable as _, FunctionTable as _};
    use fugue_core::ir::{Address, CodeBlock, Function, InsnList};
    use fugue_core::project::Project;
    use fugue_core::storage::project::DefaultTransientProjectStorageProvider;
    use prost::Message as _;
    use xz2::read::XzDecoder;

    use crate::proto::quokka;
    use crate::{DEFAULT_QUOKKA_VERSION, ExportOptions, QuokkaBuilder};

    type TestProject = Project<DefaultTransientProjectStorageProvider>;

    fn test_project() -> Result<TestProject, Box<dyn std::error::Error>> {
        Ok(Project::from_file("../fugue-core/fugue-core/tests/ls.elf")?)
    }

    fn project_with_function() -> Result<TestProject, Box<dyn std::error::Error>> {
        let mut project = test_project()?;
        let entry = Address::from(0x4000u64);
        let next = Address::from(0x4010u64);

        let first = project.blocks_mut().insert(entry, |id, start| {
            CodeBlock::try_new(id, start, 0x10, InsnList::new()).ok_or_else(|| {
                fugue_core::ir::block::table::IndexedCodeBlockTableError::other_with(
                    "invalid block",
                )
            })
        })?;
        let second = project.blocks_mut().insert(next, |id, start| {
            CodeBlock::try_new(id, start, 0x10, InsnList::new()).ok_or_else(|| {
                fugue_core::ir::block::table::IndexedCodeBlockTableError::other_with(
                    "invalid block",
                )
            })
        })?;

        let Some(block) = project.blocks_mut().get_by_id_mut(first) else {
            return Err("inserted code block not found".into());
        };
        block.add_successor(second);

        project.functions_mut().insert(entry, |id, address| {
            let mut function = Function::new(id, address);
            function.update_name("entry");
            function.add_block(entry, first);
            function.add_block(next, second);
            Ok(function)
        })?;

        Ok(project)
    }

    fn decode(bytes: &[u8]) -> Result<crate::proto::Quokka, Box<dyn std::error::Error>> {
        let mut decoder = XzDecoder::new(bytes);
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded)?;
        Ok(crate::proto::Quokka::decode(decoded.as_slice())?)
    }

    #[test]
    fn metadata_uses_hash_none() -> Result<(), Box<dyn std::error::Error>> {
        let project = test_project()?;
        let builder = QuokkaBuilder::from_project(&project)?;
        let meta = builder.quokka().meta.as_ref().ok_or("missing metadata")?;
        let hash = meta.hash.as_ref().ok_or("missing hash")?;

        assert_eq!(
            hash.hash_type,
            quokka::meta::hash::HashType::HashNone as i32
        );
        assert_eq!(hash.hash_value, "");
        assert_eq!(
            meta.calling_convention,
            quokka::CallingConvention::CcUnk as i32
        );

        let exporter_meta = builder
            .quokka()
            .exporter_meta
            .as_ref()
            .ok_or("missing exporter metadata")?;
        assert_eq!(exporter_meta.version, DEFAULT_QUOKKA_VERSION);

        Ok(())
    }

    #[test]
    fn exports_segments() -> Result<(), Box<dyn std::error::Error>> {
        let project = test_project()?;
        let builder = QuokkaBuilder::from_project(&project)?;

        assert!(!builder.quokka().segments.is_empty());
        assert!(
            builder
                .quokka()
                .segments
                .windows(2)
                .all(|segments| segments[0].virtual_addr <= segments[1].virtual_addr)
        );

        Ok(())
    }

    #[test]
    fn exports_functions_blocks_and_edges() -> Result<(), Box<dyn std::error::Error>> {
        let project = project_with_function()?;
        let builder = QuokkaBuilder::from_project_with(
            &project,
            ExportOptions::default().with_executable_name("ls.elf"),
        )?;

        let function = builder
            .quokka()
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .ok_or("missing exported function")?;

        assert_eq!(function.blocks.len(), 2);
        assert_eq!(function.edges.len(), 1);
        assert_eq!(
            function.edges[0].edge_type,
            quokka::EdgeType::EdgeJumpUncond as i32
        );

        Ok(())
    }

    #[test]
    fn writer_outputs_decodable_xz_protobuf() -> Result<(), Box<dyn std::error::Error>> {
        let project = project_with_function()?;
        let builder = QuokkaBuilder::from_project(&project)?;
        let mut bytes = Vec::new();

        builder.to_writer(&mut bytes)?;

        let decoded = decode(&bytes)?;
        assert_eq!(decoded.functions.len(), builder.quokka().functions.len());

        Ok(())
    }

    #[test]
    fn file_writer_outputs_decodable_xz_protobuf() -> Result<(), Box<dyn std::error::Error>> {
        let project = project_with_function()?;
        let builder = QuokkaBuilder::from_project(&project)?;
        let path =
            std::env::temp_dir().join(format!("fugue-core-quokka-{}.quokka", std::process::id()));

        builder.to_file(&path)?;

        let bytes = fs::read(&path)?;
        let decoded = decode(&bytes)?;
        fs::remove_file(&path)?;

        assert_eq!(decoded.segments.len(), builder.quokka().segments.len());

        Ok(())
    }
}
