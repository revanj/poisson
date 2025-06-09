use cxx::UniquePtr;

#[cxx::bridge]
mod interface {
    unsafe extern "C++" {
        include!("rust-renderer/src/slang/slang.h");
        type SlangCompilerOpaque;
        type SlangModuleOpaque;
        fn new_slang_compiler() -> UniquePtr<SlangCompilerOpaque>;
        fn load_module(self: &SlangCompilerOpaque, path_name: &str) -> UniquePtr<SlangModuleOpaque>;
    }
}

pub struct SlangCompiler {
    pub compiler_ptr: UniquePtr<interface::SlangCompilerOpaque>,
}

impl SlangCompiler {
    pub fn new() -> Self {
        let slang_compiler = interface::new_slang_compiler();
        Self {
            compiler_ptr: slang_compiler
        }
    }

    pub fn load_module(self: &Self, path_name: &str) {
        self.compiler_ptr.as_ref().unwrap().load_module(path_name);
    }
}

