mod shader_cursor;

use ash::Entry;
use cxx::UniquePtr;
use crate::slang::interface::{SlangComponentListOpaque, SlangComponentOpaque, SlangEntryPointOpaque, SlangModuleOpaque};

#[cxx::bridge]
mod interface {
    unsafe extern "C++" {
        include!("rust-renderer/src/slang/slang.h");
        type SlangEntryPointOpaque;
        type SlangModuleOpaque;
        type SlangComponentListOpaque;
        type SlangComponentOpaque;
        type SlangCompilerOpaque;
        fn new_slang_compiler() -> UniquePtr<SlangCompilerOpaque>;
        fn load_module(self: &SlangCompilerOpaque, path_name: &str) -> UniquePtr<SlangModuleOpaque>;
        fn add_module(self: Pin<&mut SlangComponentListOpaque>, module: UniquePtr<SlangModuleOpaque>);
        fn add_entry_point(self: Pin<&mut SlangComponentListOpaque>, entry_point: UniquePtr<SlangEntryPointOpaque>);
        fn compose(self: &SlangCompilerOpaque, list: UniquePtr<SlangComponentListOpaque>) -> UniquePtr<SlangComponentOpaque>;
        fn link(self: &SlangCompilerOpaque, composed: UniquePtr<SlangComponentOpaque>) -> UniquePtr<SlangComponentOpaque>;
        fn find_entry_point_by_name(self: &SlangModuleOpaque, fn_name: &str) -> UniquePtr<SlangEntryPointOpaque>;
        fn new_slang_component_list() -> UniquePtr<SlangComponentListOpaque>;
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

    pub fn link(self: &Self, composed: ComposedProgram) -> LinkedProgram {
        LinkedProgram {
            linked_program_ptr: self.compiler_ptr.as_ref().unwrap().link(composed.composed_program_ptr)
        }
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
    pub linked_program_ptr: UniquePtr<SlangComponentOpaque>
}

pub struct Module {
    pub module_ptr: UniquePtr<SlangModuleOpaque>
}

impl Module {
    pub fn find_entry_point_by_name(self: &Self, fn_name: &str) -> Option<EntryPoint> {
        let opaque_entry = self.module_ptr.as_ref().unwrap().find_entry_point_by_name(fn_name);
        Some(EntryPoint { entry_ptr: opaque_entry })
    }
}




