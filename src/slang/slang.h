
#pragma once

#include "rust-renderer/external/slang/include/slang.h"
#include "rust-renderer/external/slang/include/slang-com-ptr.h"
#include "rust-renderer/external/slang/include/slang-com-helper.h"
#include "rust/cxx.h"

#include <memory>
#include <string>

class SlangModuleOpaque {
public:
    SlangModuleOpaque(Slang::ComPtr<slang::IModule> mod, Slang::ComPtr<slang::IBlob> blob);
private:
    Slang::ComPtr<slang::IModule> module;
    Slang::ComPtr<slang::IBlob> diagnostics_blob;
};

class SlangCompilerOpaque {
public:
    SlangCompilerOpaque();
    std::unique_ptr<SlangModuleOpaque> load_module(rust::Str path_name) const;
private:
    Slang::ComPtr<slang::IGlobalSession> globalSession;
    Slang::ComPtr<slang::ISession> session;
};



std::unique_ptr<SlangCompilerOpaque> new_slang_compiler();
