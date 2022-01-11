use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt, mem,
    path::{Path, PathBuf},
};

use heck::*;
use wit_bindgen_gen_core::wit_parser::abi::{
    AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmType, WitxInstruction,
};
use wit_bindgen_gen_core::{wit_parser::*, Direction, Files, Generator, Ns};

use crate::{
    java_deps::{self, JavaDep},
    source::Source,
    ty::{JavaTupleType, JavaType},
    Opts,
};

#[derive(Default)]
pub struct WasmtimeJava {
    src: Source,
    in_import: bool,
    opts: Opts,
    host_imports: HashMap<String, HostImports>,
    host_exports: HashMap<String, HostExports>,
    sizes: SizeAlign,
    needs_clamp: bool,
    needs_store: bool,
    needs_load: bool,
    needs_validate_guest_char: bool,
    needs_expected: bool,
    needs_i32_to_f32: bool,
    needs_f32_to_i32: bool,
    needs_i64_to_f64: bool,
    needs_f64_to_i64: bool,
    needs_decode_utf8: bool,
    needs_encode_utf8: bool,
    needs_list_canon_lift: bool,
    needs_list_canon_lower: bool,
    needs_push_buffer: bool,
    needs_pull_buffer: bool,
    needs_t_typevar: bool,
    java_imports: BTreeSet<JavaDep>,
}

#[derive(Default)]
struct HostExports {
    freestanding_funcs: Vec<HostExport>,
    resource_funcs: BTreeMap<ResourceId, Vec<HostExport>>,
}

struct HostExport {
    name: String,
    src: Source,
    wasm_ty: String,
    java_sig: String,
}

#[derive(Default)]
struct HostImports {
    freestanding_funcs: Vec<Source>,
    resource_funcs: BTreeMap<ResourceId, Vec<Source>>,
    fields: BTreeMap<String, HostField>,
}

impl HostImports {
    fn insert_field(
        &mut self,
        name: impl Into<String>,
        java_ty: Cow<'static, str>,
        from: HostFieldFrom,
    ) {
        let name = name.into();
        self.fields
            .insert(name.clone(), HostField::from(name, java_ty, from));
    }
}

#[derive(Default, Debug)]
struct HostField {
    name: String,
    java_ty: Cow<'static, str>,
    from: HostFieldFrom,
}

impl HostField {
    fn from(name: String, java_ty: Cow<'static, str>, from: HostFieldFrom) -> Self {
        HostField {
            name,
            java_ty,
            from,
        }
    }

    fn is_from_constructor(&self) -> bool {
        matches!(self.from, HostFieldFrom::Constructor)
    }
}

#[derive(Debug)]
enum HostFieldFrom {
    Constructor,
    Function(Function),
}

impl Default for HostFieldFrom {
    fn default() -> Self {
        HostFieldFrom::Constructor
    }
}

impl WasmtimeJava {
    pub fn new() -> WasmtimeJava {
        WasmtimeJava::default()
    }

    pub fn opts(opts: Opts) -> WasmtimeJava {
        WasmtimeJava {
            opts,
            ..Default::default()
        }
    }

    /// Adds the package to the top of the java file.
    fn print_package(&mut self, imports: &HostImports) {
        self.src
            .push_lines(format!("package {};", self.opts.package));
        self.src.push_lines("\n");
    }

    /// Add all the imports used in the class
    fn print_imports(&mut self, imports: &HostImports) {
        for import in &self.java_imports {
            self.src
                .push_lines(format!("import {};", import.base_import()));
        }

        if !self.java_imports.is_empty() {
            self.src.push_lines("\n");
        }
    }

    /// Add a class signature
    fn print_class_header(&mut self, name: &str, imports: &HostImports) {
        // TODO: consider @NotThreadSafe
        self.src
            .push_lines(format!("public class {}", name.to_upper_camel_case()));
    }

    /// Adds a new block and increases indentation
    fn print_block_start(&mut self) {
        self.src.push_lines("{");
        self.src.indent();
    }

    /// Ends a block and reduced indentation
    fn print_block_end(&mut self) {
        self.src.outdent();
        self.src.push_lines("}");
        self.src.push_lines("");
    }

    fn tuple_ty(&mut self, _iface: &Interface, types: Vec<JavaType>) -> String {
        self.java_imports.insert(java_deps::javatuples);
        JavaTupleType::from(types).for_ty()
    }

    fn print_func_signature(&mut self, iface: &Interface, func: &Function) {
        // static and freestanding methods are always `static` methods in Java
        //   otherwise it will be null
        let static_final = match func.kind {
            FunctionKind::Static { .. }
            | FunctionKind::Freestanding
            | FunctionKind::Method { .. } => "final",
        };

        // return value
        let return_value: Cow<'_, str> = match func.results.len() {
            0 => "void".into(),
            1 => JavaType::from(func.results[0].1).for_fn_return(),
            _ => self
                .tuple_ty(
                    iface,
                    func.results.iter().map(|p| JavaType::from(p.1)).collect(),
                )
                .into(),
        };

        let params = "";
        let func_name = func.name.to_lower_camel_case();

        self.src.push_lines(format!(
            "public {} {} {}({})",
            static_final, return_value, func_name, params
        ));

        // self.src.push_str("def ");
        // match &func.kind {
        //     FunctionKind::Method { .. } => self.src.push_str(&func.item_name().to_snake_case()),
        //     FunctionKind::Static { .. } if !self.in_import => {
        //         self.src.push_str(&func.item_name().to_snake_case())
        //     }
        //     _ => self.src.push_str(&func.name.to_snake_case()),
        // }
        // if self.in_import {
        //     self.src.push_str("(self");
        // } else if let FunctionKind::Static { .. } = func.kind {
        //     self.src.push_str("(cls, caller: wasmtime.Store, obj: '");
        //     self.src.push_str(&iface.name.to_camel_case());
        //     self.src.push_str("'");
        // } else {
        //     self.src.push_str("(self, caller: wasmtime.Store");
        // }
        // let mut params = Vec::new();
        // for (i, (param, ty)) in func.params.iter().enumerate() {
        //     if i == 0 {
        //         if let FunctionKind::Method { .. } = func.kind {
        //             params.push("self".to_string());
        //             continue;
        //         }
        //     }
        //     self.src.push_str(", ");
        //     self.src.push_str(&param.to_snake_case());
        //     params.push(param.to_snake_case());
        //     self.src.push_str(": ");
        //     self.print_ty(iface, ty);
        // }
        // self.src.push_str(") -> ");
        // match func.results.len() {
        //     0 => self.src.push_str("None"),
        //     1 => self.print_ty(iface, &func.results[0].1),
        //     _ => self.print_tuple(iface, func.results.iter().map(|p| &p.1)),
        // }
        // params
    }

    /// Prints out the constructor for the given interface
    fn print_constructor(&mut self, name: &str, host_imports: &HostImports) {
        let params = host_imports
            .fields
            .values()
            .filter(|field| field.is_from_constructor());
        let assignments = params.clone();

        // collect all the param arguments, i.e. "WasmStore store, WasmInstance instance"
        let params = params
            .map(|field| format!("{} {}", field.java_ty, field.name.to_lower_camel_case()))
            .collect::<Vec<_>>()
            .join(", ");

        // set the field entries
        let assignments = assignments
            .map(|field| {
                format!(
                    "this.{name} = {name};",
                    name = field.name.to_lower_camel_case()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        self.src
            .push_lines(format!("public {}({})", name.to_upper_camel_case(), params));
        self.print_block_start();
        self.src.push_lines(assignments);
        self.print_block_end();
        self.src.push_lines("\n");
    }

    fn print_fields(&mut self, module: &str, host_imports: &HostImports) {
        let fields = host_imports.fields.values();

        for field in fields {
            self.src.push_lines(format!(
                "private final {ty} {name};",
                ty = field.java_ty,
                name = field.name.to_lower_camel_case()
            ));
        }
    }
}

impl Generator for WasmtimeJava {
    fn type_record(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        eprintln!("type_record: {:?}, {:?}, {}, {:?}", iface, id, name, record);
        todo!()
    }

    fn type_variant(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        eprintln!(
            "type_variant: {:?}, {:?}, {}, {:?}",
            iface, id, name, variant
        );
        todo!()
    }

    fn type_resource(&mut self, iface: &Interface, ty: ResourceId) {
        eprintln!("type_resource: {:?}, {:?}", iface, ty);
        todo!()
    }

    fn type_alias(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        eprintln!("type_alias: {:?}, {:?}, {}, {:?}", iface, id, name, ty);
        todo!()
    }

    fn type_list(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        eprintln!("type_list: {:?}, {:?}, {}, {:?}", iface, id, name, ty);
        todo!()
    }

    fn type_pointer(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        const_: bool,
        ty: &Type,
        docs: &Docs,
    ) {
        eprintln!(
            "type_pointer: {:?}, {:?}, {}, {}, {:?}",
            iface, id, name, const_, ty
        );
        todo!()
    }

    fn type_builtin(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        eprintln!("type_builtin: {:?}, {:?}, {}, {:?}", iface, id, name, ty);
        todo!()
    }

    fn type_push_buffer(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        ty: &Type,
        docs: &Docs,
    ) {
        eprintln!(
            "type_push_buffer: {:?}, {:?}, {}, {:?}",
            iface, id, name, ty
        );
        todo!()
    }

    fn type_pull_buffer(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        ty: &Type,
        docs: &Docs,
    ) {
        eprintln!(
            "type_pull_buffer: {:?}, {:?}, {}, {:?}",
            iface, id, name, ty
        );
        todo!()
    }

    // import: Interface { name: "char", types: Arena { arena_id: 0, items: [], _phantom: PhantomData }, type_lookup: {}, resources: Arena { arena_id: 1, items: [], _phantom: PhantomData }, resource_lookup: {}, interfaces: Arena { arena_id: 4, items: [], _phantom: PhantomData }, interface_lookup: {}, functions: [Function { abi: Canonical, is_async: false, docs: Docs { contents: None }, name: "take-char", kind: Freestanding, params: [("x", Char)], results: [] }, Function { abi: Canonical, is_async: false, docs: Docs { contents: None }, name: "return-char", kind: Freestanding, params: [], results: [("", Char)] }], globals: [] }, Function { abi: Canonical, is_async: false, docs: Docs { contents: None }, name: "take-char", kind: Freestanding, params: [("x", Char)], results: [] }
    // import: Interface { name: "char", types: Arena { arena_id: 0, items: [], _phantom: PhantomData }, type_lookup: {}, resources: Arena { arena_id: 1, items: [], _phantom: PhantomData }, resource_lookup: {}, interfaces: Arena { arena_id: 4, items: [], _phantom: PhantomData }, interface_lookup: {}, functions: [Function { abi: Canonical, is_async: false, docs: Docs { contents: None }, name: "take-char", kind: Freestanding, params: [("x", Char)], results: [] }, Function { abi: Canonical, is_async: false, docs: Docs { contents: None }, name: "return-char", kind: Freestanding, params: [], results: [("", Char)] }], globals: [] }, Function { abi: Canonical, is_async: false, docs: Docs { contents: None }, name: "return-char", kind: Freestanding, params: [], results: [("", Char)] }
    fn import(&mut self, iface: &Interface, func: &Function) {
        eprintln!("import: {:?}, {:?}", iface, func);

        assert!(!func.is_async, "async not supported yet");

        // using the local functions to append sections of the function, the original will be replaced later.
        //  TODO: move print methods to Source?
        let prev = mem::take(&mut self.src);

        self.print_func_signature(iface, func);
        self.print_block_start();
        self.print_block_end();

        let mut host_imports = self
            .host_imports
            .entry(iface.name.to_string())
            .or_insert_with(HostImports::default);

        // need the wasm instance
        self.java_imports.insert(java_deps::wasmtime);
        host_imports.insert_field(
            java_deps::wt_instance.to_lower_camel_case(),
            java_deps::wt_instance.into(),
            HostFieldFrom::Constructor,
        );
        host_imports.insert_field(
            java_deps::wt_store.to_lower_camel_case(),
            java_deps::wt_store.into(),
            HostFieldFrom::Constructor,
        );

        // if needs_memory {
        //     exports
        //         .fields
        //         .insert("memory".to_string(), "wasmtime.Memory");
        // }
        // if let Some(name) = &needs_realloc {
        //     exports.fields.insert(name.clone(), "wasmtime.Func");
        // }
        // if let Some(name) = &needs_free {
        //     exports.fields.insert(name.clone(), "wasmtime.Func");
        // }
        // exports.fields.insert(func.name.clone(), "wasmtime.Func");

        let func_body = mem::replace(&mut self.src, prev);

        let dst = match &func.kind {
            FunctionKind::Freestanding => &mut host_imports.freestanding_funcs,
            FunctionKind::Static { resource, .. } | FunctionKind::Method { resource, .. } => {
                host_imports
                    .resource_funcs
                    .entry(*resource)
                    .or_insert(Vec::new())
            }
        };
        dst.push(func_body);
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        eprintln!("export: {:?}, {:?}", iface, func);
        todo!()
    }

    // finish_one: Interface { name: "char", types: Arena { arena_id: 0, items: [], _phantom: PhantomData }, type_lookup: {}, resources: Arena { arena_id: 1, items: [], _phantom: PhantomData }, resource_lookup: {}, interfaces: Arena { arena_id: 4, items: [], _phantom: PhantomData }, interface_lookup: {}, functions: [Function { abi: Canonical, is_async: false, docs: Docs { contents: None }, name: "take-char", kind: Freestanding, params: [("x", Char)], results: [] }, Function { abi: Canonical, is_async: false, docs: Docs { contents: None }, name: "return-char", kind: Freestanding, params: [], results: [("", Char)] }], globals: [] }
    fn finish_one(&mut self, iface: &Interface, files: &mut Files) {
        eprintln!("finish_one: {:?}", iface);

        // print all classes & functions
        for (module, funcs) in mem::take(&mut self.host_imports) {
            // top of the file needs to be the enclosing package
            self.print_package(&funcs);

            self.print_imports(&funcs);

            // begin the class
            self.print_class_header(&module, &funcs);
            self.print_block_start();

            self.print_fields(&module, &funcs);
            self.src.push_lines("\n");

            self.print_constructor(&module, &funcs);

            for func in funcs.freestanding_funcs.iter() {
                self.src.push_lines(func.as_str());
                self.src.push_lines("\n");
            }

            self.print_block_end();

            // the java file path and package
            let mut path = PathBuf::from(self.opts.package.replace(".", "/"));
            let java_class_name = iface.name.to_upper_camel_case();
            path.push(java_class_name);
            path.set_extension("java");

            files.push(&path.to_string_lossy(), self.src.as_bytes());
        }
    }
}
