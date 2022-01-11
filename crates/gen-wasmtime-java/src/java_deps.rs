pub const javatuples: JavaDep =
    JavaDep::def("org.javatuples.*", "org.javatuples", "javatuples", "1.2");
pub const wasmtime: JavaDep = JavaDep::def(
    "net.bluejekyll.wasmtime.*",
    "net.bluejekyll",
    "wasmtime-java",
    "1.0-SNAPSHOT",
);

pub const wt_engine: &str = "WasmEngine";
pub const wt_store: &str = "WasmStore";
pub const wt_instance: &str = "WasmInstance";
pub const wt_function: &str = "WasmFunction";
pub const wt_module: &str = "WasmModule";

#[derive(Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct JavaDep {
    base_import: &'static str,
    group_id: &'static str,
    artifact_id: &'static str,
    version: &'static str,
}

impl JavaDep {
    pub const fn def(
        base_import: &'static str,
        group_id: &'static str,
        artifact_id: &'static str,
        version: &'static str,
    ) -> Self {
        Self {
            base_import,
            group_id,
            artifact_id,
            version,
        }
    }

    /// Returns the base_import with a wildcard, e.g. `org.javatuples.*`
    pub fn base_import(&self) -> &str {
        self.base_import
    }

    // /// Appends (replaces the `*`) to the base_import for this dependence
    // pub fn import(&self, class_path: &str) -> String {
    //     self.base_import.replace("*", class_path)
    // }

    /// Get the pom group id
    pub fn group_id(&self) -> &str {
        self.group_id
    }

    /// Get the pom artifacti id
    pub fn artifact_id(&self) -> &str {
        self.artifact_id
    }

    /// Get the library version
    pub fn version(&self) -> &str {
        self.version
    }

    /// Returns a string of the Pom dependency
    pub fn pom_dep(&self) -> String {
        format!(
            "
        <dependency>
            <groupId>{group_id}</groupId>
            <artifactId>{artifact_id}</artifactId>
            <version>{version}</version>
        </dependency>
        ",
            group_id = self.group_id,
            artifact_id = self.artifact_id,
            version = self.version
        )
    }
}
