mod shader_cursor;

use std::fmt;
use std::fmt::{Debug, Formatter};
use ash::Entry;
use cxx::UniquePtr;
use crate::slang::interface::{ShaderStage, SlangByteCodeOpaque, SlangComponentListOpaque, SlangComponentOpaque, SlangEntryPointOpaque, SlangModuleOpaque, VarType};
pub use interface::SlangEntryPointReflection;
pub use interface::SlangProgramReflection;
pub use interface::SlangStructReflection;
pub use interface::SlangFieldReflection;



#[cfg(target_arch = "x86_64")]
#[cxx::bridge]
mod interface {

    #[derive(Debug)]
    pub enum VarType {
        Undefined,
        Float,
        Float2,
        Float3,
        Float4,
    }

    #[derive(Debug)]
    enum ShaderStage {
        None,
        Vertex,
        Fragment,
        Compute,
    }

    #[derive(Debug)]
    struct SlangEntryPointReflection {
        name: String,
        stage: ShaderStage,
        // everything wrapped in a struct will be structs
        struct_reflections: Vec<SlangStructReflection>,

        // everything not wrapped in a struct will be in a spare, extra struct
        misc_reflections: SlangStructReflection,
    }

    #[derive(Debug)]
    struct SlangProgramReflection {
        // this also assumes uniforms are a single struct of primitives
        // which is mostly fine
        uniform_reflections: Vec<SlangStructReflection>,
        entry_point_reflections: Vec<SlangEntryPointReflection>
    }

    #[derive(Debug)]
    struct SlangStructReflection {
        name: String,
        binding: u32,
        fields: Vec<SlangFieldReflection>,
    }

    // a fully general data desc recursive enum class is a bit annoying
    #[derive(Debug)]
    struct SlangFieldReflection {
        name: String,
        location: u32,
        var_type: VarType
    }

    unsafe extern "C++" {
        include!("rust-renderer/src/slang/slang.h");

        type SlangEntryPointOpaque;
        type SlangModuleOpaque;
        type SlangByteCodeOpaque;
        type SlangComponentListOpaque;
        type SlangComponentOpaque;
        type SlangCompilerOpaque;

        fn new_slang_compiler() -> UniquePtr<SlangCompilerOpaque>;
        fn load_module(self: &SlangCompilerOpaque, path_name: &str) -> UniquePtr<SlangModuleOpaque>;
        fn add_module(self: Pin<&mut SlangComponentListOpaque>, module: UniquePtr<SlangModuleOpaque>);
        fn add_entry_point(self: Pin<&mut SlangComponentListOpaque>, entry_point: UniquePtr<SlangEntryPointOpaque>);
        fn compose(self: &SlangCompilerOpaque, list: UniquePtr<SlangComponentListOpaque>) -> UniquePtr<SlangComponentOpaque>;
        fn link(self: &SlangCompilerOpaque, composed: UniquePtr<SlangComponentOpaque>) -> UniquePtr<SlangComponentOpaque>;
        fn link_module(self: &SlangCompilerOpaque, module: UniquePtr<SlangModuleOpaque>) -> UniquePtr<SlangComponentOpaque>;
        fn find_entry_point_by_name(self: &SlangModuleOpaque, fn_name: &str) -> UniquePtr<SlangEntryPointOpaque>;
        fn get_entry_point_count(self: &SlangModuleOpaque) -> u32;
        fn get_entry_point_by_index(self: &SlangModuleOpaque, idx: u32) -> UniquePtr<SlangEntryPointOpaque>;
        fn get_bytes(self: &SlangByteCodeOpaque) -> &[u32];
        fn get_target_code(self: &SlangComponentOpaque) -> UniquePtr<SlangByteCodeOpaque>;
        fn new_slang_component_list() -> UniquePtr<SlangComponentListOpaque>;
        fn get_program_reflection(self: &SlangComponentOpaque) -> SlangProgramReflection;
    }
}

impl fmt::Display for VarType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for SlangStructReflection {
    fn fmt(self: &Self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut ret_string = String::new();
        ret_string.push_str("binding: ");
        ret_string.push_str(&self.binding.to_string());
        ret_string.push_str(", ");
        ret_string.push_str(&self.name);
        ret_string.push_str(": ");
        for field in &self.fields {
            ret_string.push_str("location: ");
            ret_string.push_str(&field.location.to_string());
            ret_string.push_str(", ");
            ret_string.push_str(&field.var_type.to_string());
            ret_string.push(' ');
            ret_string.push_str(&field.name);
            ret_string.push_str(", ");
        }

        write!(f, "{}", ret_string)
    }
}

pub struct Compiler {
    pub compiler_ptr: UniquePtr<interface::SlangCompilerOpaque>,
}

impl Compiler {
    pub fn new() -> Self {
        let slang_compiler = interface::new_slang_compiler();
        Self {
            compiler_ptr: slang_compiler
        }
    }

    pub fn load_module(self: &Self, path_name: &str) -> Module {
        Module {
            module_ptr: self.compiler_ptr.as_ref().unwrap().load_module(path_name)
        }
    }

    pub fn compose_components(self: &Self, components: ComponentList) -> ComposedProgram {
        ComposedProgram {
            composed_program_ptr: self.compiler_ptr.as_ref().unwrap().compose(components.components)
        }
    }

    pub fn link_composed_program(self: &Self, composed: ComposedProgram) -> LinkedProgram {
        LinkedProgram::new(
            self.compiler_ptr.as_ref().unwrap()
                .link(composed.composed_program_ptr)
        )
    }

    pub fn link_module(self: &Self, module: Module) -> LinkedProgram {
        LinkedProgram::new(
            self.compiler_ptr.as_ref().unwrap().link_module(module.module_ptr)
        )
    }

    pub fn linked_program_from_file(self: &Self, path: &str) -> LinkedProgram {
        let module = self.load_module(path);
        let mut to_compose = ComponentList::new();
        for idx in 0..module.get_entry_point_count() {
            let entry = module.get_entry_point_by_index(idx);
            to_compose.add_entry_point(entry.unwrap());
        }
        to_compose.add_module(module);
        let composed = self.compose_components(to_compose);
        self.link_composed_program(composed)
    }
}

pub struct EntryPoint {
    pub entry_ptr: UniquePtr<SlangEntryPointOpaque>
}

pub struct ComponentList {
    pub components: UniquePtr<SlangComponentListOpaque>
}

impl ComponentList {
    pub fn new() -> Self {
        let slang_component_list = interface::new_slang_component_list();
        Self {
            components: slang_component_list
        }
    }
}

impl ComponentList {
    pub fn add_module(self: &mut Self, module: Module) {
        self.components.as_mut().unwrap().add_module(module.module_ptr);
    }

    pub fn add_entry_point(self: &mut Self, entry: EntryPoint) {
        self.components.as_mut().unwrap().add_entry_point(entry.entry_ptr);
    }
}

pub struct ComposedProgram {
    pub composed_program_ptr: UniquePtr<SlangComponentOpaque>
}

pub struct LinkedProgram {
    pub linked_program_ptr: UniquePtr<SlangComponentOpaque>,
    pub byte_code_ptr: UniquePtr<SlangByteCodeOpaque>,
}

impl LinkedProgram {
    pub fn new(linked_ptr: UniquePtr<SlangComponentOpaque>) -> Self {
        let byte_ptr = linked_ptr.as_ref().unwrap().get_target_code();
        Self {
            linked_program_ptr: linked_ptr,
            byte_code_ptr: byte_ptr
        }
    }
    pub fn get_bytecode(self: &Self) -> &[u32] {
        self.byte_code_ptr.as_ref().unwrap().get_bytes()
    }

    pub fn get_reflection(self: &Self) -> SlangProgramReflection {
        self.linked_program_ptr.get_program_reflection()
    }
}


pub struct Module {
    pub module_ptr: UniquePtr<SlangModuleOpaque>
}

impl Module {
    pub fn find_entry_point_by_name(self: &Self, fn_name: &str) -> Option<EntryPoint> {
        let opaque_entry = self.module_ptr.as_ref().unwrap().find_entry_point_by_name(fn_name);
        if opaque_entry.is_null() {
            None
        } else {
            Some(EntryPoint { entry_ptr: opaque_entry })
        }
    }

    pub fn get_entry_point_count(self: &Self) -> u32 {
        self.module_ptr.as_ref().unwrap().get_entry_point_count()
    }

    pub fn get_entry_point_by_index(self: &Self, idx: u32) -> Option<EntryPoint> {
        let opaque_entry = self.module_ptr.as_ref().unwrap().get_entry_point_by_index(idx);
        if opaque_entry.is_null() {
            None
        } else {
            Some(EntryPoint { entry_ptr: opaque_entry })
        }
    }
}




