use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use fugue_core::project::Project;
use prost::Message as _;
use prost::bytes::BytesMut;
use xz2::write::XzEncoder;

use crate::error::QuokkaBuilderError;
use crate::export::functions::FunctionExporter;
use crate::export::metadata::MetadataExporter;
use crate::export::segments::SegmentExporter;
use crate::options::ExportOptions;
use crate::proto;

pub struct QuokkaBuilder {
    quokka: proto::Quokka,
    options: ExportOptions,
}

impl QuokkaBuilder {
    pub fn from_project(project: &Project) -> Result<Self, QuokkaBuilderError> {
        Self::from_project_with(project, ExportOptions::default())
    }

    pub fn from_project_with(
        project: &Project,
        options: ExportOptions,
    ) -> Result<Self, QuokkaBuilderError> {
        let mut quokka = proto::Quokka {
            meta: Some(MetadataExporter::meta(project, &options)),
            exporter_meta: Some(MetadataExporter::exporter_meta(&options)),
            layout: Vec::new(),
            data: Vec::new(),
            types: Vec::new(),
            instructions: Vec::new(),
            mnemonics: Vec::new(),
            functions: Vec::new(),
            references: Vec::new(),
            register_table: Vec::new(),
            operands: Vec::new(),
            segments: Vec::new(),
            orphaned_instructions: Vec::new(),
            operand_strings: Vec::new(),
            headers: String::new(),
        };

        let segments = SegmentExporter::populate(project, &mut quokka)?;
        FunctionExporter::populate(project, &segments, &mut quokka)?;

        Ok(Self { quokka, options })
    }

    pub fn quokka(&self) -> &proto::Quokka {
        &self.quokka
    }

    pub fn quokka_mut(&mut self) -> &mut proto::Quokka {
        &mut self.quokka
    }

    pub fn into_quokka(self) -> proto::Quokka {
        self.quokka
    }

    pub fn options(&self) -> &ExportOptions {
        &self.options
    }

    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<(), QuokkaBuilderError> {
        let path = path.as_ref();
        let file = File::create(path)
            .map_err(|source| QuokkaBuilderError::io("create file", path, source))?;
        let writer = BufWriter::new(file);
        self.to_writer(writer)
            .map_err(|error| error.with_path("write file", path))
    }

    pub fn to_writer(&self, writer: impl Write) -> Result<(), QuokkaBuilderError> {
        let mut bytes = BytesMut::new();
        self.quokka.encode(&mut bytes)?;

        let mut encoder = XzEncoder::new(writer, self.options.compression_level());
        encoder
            .write_all(bytes.as_ref())
            .map_err(QuokkaBuilderError::Stream)?;
        encoder.finish().map_err(QuokkaBuilderError::Stream)?;

        Ok(())
    }
}
