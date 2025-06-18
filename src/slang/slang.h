
#pragma once

#include "rust-renderer/external/slang/include/slang.h"
#include "rust-renderer/external/slang/include/slang-com-ptr.h"
#include "rust-renderer/external/slang/include/slang-com-helper.h"
#include "rust/cxx.h"

#include <memory>
#include <string>
#include <vector>
#include <cstdint>

class SlangEntryPointOpaque {
public:
    SlangEntryPointOpaque(Slang::ComPtr<slang::IEntryPoint> entry);
    Slang::ComPtr<slang::IEntryPoint> entry_point;
};

class SlangModuleOpaque {
public:
    SlangModuleOpaque(Slang::ComPtr<slang::IModule> mod, Slang::ComPtr<slang::IBlob> blob);
    std::unique_ptr<SlangEntryPointOpaque> find_entry_point_by_name(rust::Str name) const;
    Slang::ComPtr<slang::IModule> module;
    Slang::ComPtr<slang::IBlob> diagnostics_blob;
};

class SlangByteCodeOpaque {
public:
    SlangByteCodeOpaque(Slang::ComPtr<slang::IBlob> c, Slang::ComPtr<slang::IBlob> blob);
    rust::Slice<const uint32_t> get_bytes() const;
    Slang::ComPtr<slang::IBlob> code;
    Slang::ComPtr<slang::IBlob> diagnostics_blob;
};

// a little wrapper class that avoids dyn or unsafe in rust
// accepts unique pointers to shader components (modules and entry points)
class SlangComponentListOpaque {
public:
    std::vector<slang::IComponentType*> components;
    void add_module(std::unique_ptr<SlangModuleOpaque> module);
    void add_entry_point(std::unique_ptr<SlangEntryPointOpaque> entry_point);
};

class SlangComponentOpaque {
public:
    SlangComponentOpaque(Slang::ComPtr<slang::IComponentType> comp, Slang::ComPtr<slang::IBlob> blob);
    std::unique_ptr<SlangByteCodeOpaque> get_target_code() const;
    Slang::ComPtr<slang::IComponentType> component;
    Slang::ComPtr<slang::IBlob> diagnostics_blob;
};

class SlangCompilerOpaque {
public:
    SlangCompilerOpaque();
    std::unique_ptr<SlangModuleOpaque> load_module(rust::Str path_name) const;
    std::unique_ptr<SlangComponentOpaque> compose(std::unique_ptr<SlangComponentListOpaque> list) const;
    std::unique_ptr<SlangComponentOpaque> link(std::unique_ptr<SlangComponentOpaque> composed) const;
    std::unique_ptr<SlangComponentOpaque> link_module(std::unique_ptr<SlangModuleOpaque> module) const;

private:
    Slang::ComPtr<slang::IGlobalSession> globalSession;
    Slang::ComPtr<slang::ISession> session;
};

std::unique_ptr<SlangComponentListOpaque> new_slang_component_list();
std::unique_ptr<SlangCompilerOpaque> new_slang_compiler();
void reflection_test();
