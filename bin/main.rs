use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser;
use clap::builder::NonEmptyStringValueParser;
use fugue_core::analysis::AnalysisPass;
use fugue_core::ir::RawAddress;
use fugue_core::loader::{Loadable, LoadableAnalysers, Loader};
use fugue_core::project::Project;
use fugue_core_quokka::{ExportOptions, QuokkaBuilder};

#[derive(Parser)]
#[command(about = "Export a Fugue project to a Quokka protobuf database")]
struct Cli {
    /// Input binary
    input: PathBuf,
    /// Output .quokka file
    output: PathBuf,
    /// Executable name stored in Quokka metadata
    #[arg(long, value_name = "NAME", value_parser = NonEmptyStringValueParser::new())]
    executable_name: Option<String>,
    /// Quokka exporter metadata version
    #[arg(long, value_name = "VERSION", value_parser = NonEmptyStringValueParser::new())]
    quokka_version: Option<String>,
    /// Backend version stored in Quokka metadata
    #[arg(long, value_name = "VERSION", value_parser = NonEmptyStringValueParser::new())]
    backend_version: Option<String>,
    /// XZ compression level
    #[arg(long, value_name = "LEVEL", default_value_t = 6, value_parser = clap::value_parser!(u32).range(0..=9))]
    compression_level: u32,
    /// Run Fugue function recovery before export
    #[arg(long)]
    recover_functions: bool,
    /// Seed function recovery with an explicit function start
    #[arg(long = "function-candidate", value_name = "ADDRESS")]
    function_candidates: Vec<RawAddress>,
}

impl Cli {
    fn default_executable_name(input: &Path) -> String {
        input
            .file_name()
            .and_then(OsStr::to_str)
            .map(str::to_owned)
            .unwrap_or_default()
    }

    fn export_options(&self) -> ExportOptions {
        let executable_name = self
            .executable_name
            .as_deref()
            .map(str::to_owned)
            .unwrap_or_else(|| Self::default_executable_name(&self.input));
        let mut options = ExportOptions::default()
            .with_executable_name(&executable_name)
            .with_compression_level(self.compression_level);

        if let Some(quokka_version) = &self.quokka_version {
            options.set_quokka_version(quokka_version);
        }

        if let Some(backend_version) = &self.backend_version {
            options.set_backend_version(backend_version);
        }

        options
    }

    fn run(&self) -> Result<()> {
        let loader = Loader::from_file(&self.input)?;
        let mut project = Project::new_transient(&loader)?;

        if self.recover_functions || !self.function_candidates.is_empty() {
            let mut recovery = loader.analysers().function_recovery()?;

            for candidate in &self.function_candidates {
                recovery.add_candidate(*candidate);
            }

            recovery.analyse(&mut project)?;
        }

        let builder = QuokkaBuilder::from_project_with(&project, self.export_options())?;
        builder.to_file(&self.output)?;

        Ok(())
    }
}

fn main() -> Result<()> {
    Cli::parse().run()
}
