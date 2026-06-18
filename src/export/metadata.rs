use fugue_core::ir::Endian;
use fugue_core::project::Project;

use crate::options::ExportOptions;
use crate::proto::quokka;

pub(crate) struct MetadataExporter;

impl MetadataExporter {
    pub(crate) fn exporter_meta(options: &ExportOptions) -> quokka::ExporterMeta {
        quokka::ExporterMeta {
            mode: quokka::exporter_meta::Mode::Light as i32,
            version: options.quokka_version().to_owned(),
        }
    }

    pub(crate) fn meta(project: &Project, options: &ExportOptions) -> quokka::Meta {
        quokka::Meta {
            executable_name: options.executable_name().to_owned(),
            isa: Self::isa(project) as i32,
            calling_convention: quokka::CallingConvention::CcUnk as i32,
            hash: Some(quokka::meta::Hash {
                hash_type: quokka::meta::hash::HashType::HashNone as i32,
                hash_value: String::new(),
            }),
            endianess: Self::endianess(project) as i32,
            address_size: Self::address_size(project) as i32,
            backend: Some(quokka::meta::Backend {
                name: quokka::meta::backend::Disassembler::DisassUnknown as i32,
                version: options.backend_version().to_owned(),
            }),
            decompilation_activated: false,
        }
    }

    pub(crate) fn address_size(project: &Project) -> quokka::AddressSize {
        match project.language().address_bits() {
            32 => quokka::AddressSize::Addr32,
            64 => quokka::AddressSize::Addr64,
            _ => quokka::AddressSize::AddrUnk,
        }
    }

    fn isa(project: &Project) -> quokka::meta::Isa {
        match project.language().processor() {
            "x86" => quokka::meta::Isa::ProcIntel,
            "ARM" | "AARCH64" => quokka::meta::Isa::ProcArm,
            "MIPS" => quokka::meta::Isa::ProcMips,
            _ => quokka::meta::Isa::ProcUnk,
        }
    }

    fn endianess(project: &Project) -> quokka::meta::Endianess {
        match project.arch().endian() {
            Endian::Little => quokka::meta::Endianess::EndLe,
            Endian::Big => quokka::meta::Endianess::EndBe,
        }
    }
}
