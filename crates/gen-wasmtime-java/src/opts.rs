use crate::WasmtimeJava;

const JAVA_PACKAGE_DFLT: &str = "bindings";

/// Generates the bindings file for the specified WIT. The generated Java can be used in the context of the wasmtime-java project.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    /// Package for the Java bindings file
    #[cfg_attr(feature = "structopt", structopt(long = "package", default_value = JAVA_PACKAGE_DFLT))]
    pub package: String,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            package: JAVA_PACKAGE_DFLT.to_string(),
        }
    }
}

impl Opts {
    pub fn build(self) -> WasmtimeJava {
        WasmtimeJava::opts(self)
    }
}
