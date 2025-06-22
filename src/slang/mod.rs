mod shader_cursor;

use ash::Entry;
use cxx::UniquePtr;
use crate::slang::interface::{SlangByteCodeOpaque, SlangComponentListOpaque, SlangComponentOpaque, SlangEntryPointOpaque, SlangModuleOpaque};
pub use interface::SlangEntryPointReflection;
pub use interface::SlangProgramReflection;
pub use interface::SlangParamReflection;

#[cxx::bridge]
mod interface {
    pub enum VarType {
        Float,
        Vec2_Float,
        Vec3_Float,
        Vec4_Float,
    }

    #[derive(Debug)]
    enum ShaderStage {
        None,
        Vertex,
        Fragment,
        Compute,
    }
    struct SlangEntryPointReflection {
        name: String,
        stage: ShaderStage,
        // this assumes input is a single struct of primitives
        param_reflections: Vec<SlangParamReflection>,
    }

    struct SlangProgramReflection {
        // this also assumes uniforms are a single struct of primitives
        // which is mostly fine
        uniform_reflections: Vec<SlangParamReflection>,
        entry_point_reflections: Vec<SlangEntryPointReflection>
    }

    // a fully general data desc recursive enum class is a bit annoying
    struct SlangParamReflection {
        name: String,
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

        self.link_module(module)
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




