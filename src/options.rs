pub const DEFAULT_QUOKKA_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOptions {
    executable_name: String,
    quokka_version: String,
    backend_version: String,
    compression_level: u32,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            executable_name: String::new(),
            quokka_version: DEFAULT_QUOKKA_VERSION.to_owned(),
            backend_version: env!("CARGO_PKG_VERSION").to_owned(),
            compression_level: 6,
        }
    }
}

impl ExportOptions {
    pub fn executable_name(&self) -> &str {
        &self.executable_name
    }

    pub fn set_executable_name(&mut self, executable_name: impl AsRef<str>) {
        self.executable_name = executable_name.as_ref().to_owned();
    }

    pub fn with_executable_name(mut self, executable_name: impl AsRef<str>) -> Self {
        self.set_executable_name(executable_name);
        self
    }

    pub fn quokka_version(&self) -> &str {
        &self.quokka_version
    }

    pub fn set_quokka_version(&mut self, quokka_version: impl AsRef<str>) {
        self.quokka_version = quokka_version.as_ref().to_owned();
    }

    pub fn with_quokka_version(mut self, quokka_version: impl AsRef<str>) -> Self {
        self.set_quokka_version(quokka_version);
        self
    }

    pub fn backend_version(&self) -> &str {
        &self.backend_version
    }

    pub fn set_backend_version(&mut self, backend_version: impl AsRef<str>) {
        self.backend_version = backend_version.as_ref().to_owned();
    }

    pub fn with_backend_version(mut self, backend_version: impl AsRef<str>) -> Self {
        self.set_backend_version(backend_version);
        self
    }

    pub fn compression_level(&self) -> u32 {
        self.compression_level
    }

    pub fn set_compression_level(&mut self, compression_level: u32) {
        self.compression_level = compression_level;
    }

    pub fn with_compression_level(mut self, compression_level: u32) -> Self {
        self.set_compression_level(compression_level);
        self
    }
}
