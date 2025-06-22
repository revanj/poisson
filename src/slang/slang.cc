#include "rust-renderer/src/slang/slang.h"

#include "rust-renderer/external/slang/include/slang.h"
#include "rust-renderer/external/slang/include/slang-com-ptr.h"
#include "rust-renderer/external/slang/include/slang-com-helper.h"

#include "rust-renderer/src/slang/mod.rs.h"

#include <iostream>
#include <array>


SlangEntryPointOpaque::SlangEntryPointOpaque(
    Slang::ComPtr<slang::IEntryPoint> entry):
        entry_point(entry) {}

SlangModuleOpaque::SlangModuleOpaque(Slang::ComPtr<slang::IModule> mod, Slang::ComPtr<slang::IBlob> blob)
    : module(mod), diagnostics_blob(blob) {}

std::unique_ptr<SlangEntryPointOpaque> SlangModuleOpaque::find_entry_point_by_name(rust::Str name) const {
    Slang::ComPtr<slang::IEntryPoint> entry;
    module->findEntryPointByName(((std::string)name).c_str(), entry.writeRef());

    if (!entry){
       std::cout << "no entry point!" << std::endl;
    }

    return std::unique_ptr<SlangEntryPointOpaque>(new SlangEntryPointOpaque(entry));
}

uint32_t SlangModuleOpaque::get_entry_point_count() const {
    return module->getDefinedEntryPointCount();
}

std::unique_ptr<SlangEntryPointOpaque> SlangModuleOpaque::get_entry_point_by_index(uint32_t idx) const {
    Slang::ComPtr<slang::IEntryPoint> entry;
    module->getDefinedEntryPoint(idx, entry.writeRef());

    if (!entry) {
        std::cout << "no entry point!" << std::endl;
        return std::unique_ptr<SlangEntryPointOpaque>(nullptr);
    }

    return std::unique_ptr<SlangEntryPointOpaque>(new SlangEntryPointOpaque(entry));
}

SlangByteCodeOpaque::SlangByteCodeOpaque(Slang::ComPtr<slang::IBlob> c, Slang::ComPtr<slang::IBlob> blob):
    code(c), diagnostics_blob(blob) {}

rust::Slice<const uint32_t> SlangByteCodeOpaque::get_bytes() const {
    uint32_t const* buffer_start = static_cast<uint32_t const*>(code->getBufferPointer());
    uint32_t buffer_size = static_cast<uint32_t>(code->getBufferSize()) / 4;

    return rust::Slice<const uint32_t>(buffer_start, buffer_size);
}

void SlangComponentListOpaque::add_module(std::unique_ptr<SlangModuleOpaque> module) {
    components.push_back(module->module);
}

void SlangComponentListOpaque::add_entry_point(std::unique_ptr<SlangEntryPointOpaque> entry_point) {
    components.push_back(entry_point->entry_point);
}

SlangComponentOpaque::SlangComponentOpaque(Slang::ComPtr<slang::IComponentType> comp, Slang::ComPtr<slang::IBlob> blob):
    component(comp), diagnostics_blob(blob) {}

std::unique_ptr<SlangByteCodeOpaque> SlangComponentOpaque::get_target_code() const {
    Slang::ComPtr<slang::IBlob> code;
    Slang::ComPtr<slang::IBlob> blob;
    component->getTargetCode(0, code.writeRef(), blob.writeRef());
    return std::unique_ptr<SlangByteCodeOpaque>(new SlangByteCodeOpaque(code, blob));
}

SlangCompilerOpaque::SlangCompilerOpaque() {
    createGlobalSession(globalSession.writeRef());
    slang::SessionDesc sessionDesc = {};
    slang::TargetDesc targetDesc = {};
    targetDesc.format = SLANG_SPIRV;
    targetDesc.profile = globalSession->findProfile("spirv_1_0");

    sessionDesc.targets = &targetDesc;
    sessionDesc.targetCount = 1;

    std::array<slang::CompilerOptionEntry, 1> options =
    {
        {
            slang::CompilerOptionName::EmitSpirvDirectly,
            { slang::CompilerOptionValueKind::Int, 1, 0, nullptr, nullptr }
        }
    };
    sessionDesc.compilerOptionEntries = options.data();
    sessionDesc.compilerOptionEntryCount = options.size();

    globalSession->createSession(sessionDesc, session.writeRef());
}


std::unique_ptr<SlangModuleOpaque> SlangCompilerOpaque::load_module(rust::Str path_name) const {
    Slang::ComPtr<slang::IModule> mod;
    Slang::ComPtr<slang::IBlob> blob;
    mod = session->loadModule(((std::string)path_name).c_str(), blob.writeRef());
    if (blob != nullptr)
    {
        std::cout << (const char*)blob->getBufferPointer() << std::endl;
    } else {
        std::cout << "successfully compiled module" << std::endl;
    }
    return std::unique_ptr<SlangModuleOpaque>(new SlangModuleOpaque(mod, blob));
}

std::unique_ptr<SlangComponentOpaque> SlangCompilerOpaque::compose(std::unique_ptr<SlangComponentListOpaque> list) const {
    Slang::ComPtr<slang::IComponentType> composedProgram;
    Slang::ComPtr<slang::IBlob> diagnosticsBlob;
    session->createCompositeComponentType(
        list->components.data(),
        list->components.size(),
        composedProgram.writeRef(),
        diagnosticsBlob.writeRef());

    return std::unique_ptr<SlangComponentOpaque>(new SlangComponentOpaque(composedProgram, diagnosticsBlob));
}

std::unique_ptr<SlangComponentOpaque> SlangCompilerOpaque::link(std::unique_ptr<SlangComponentOpaque> composed) const {
    Slang::ComPtr<slang::IComponentType> linkedProgram;
    Slang::ComPtr<slang::IBlob> diagnosticsBlob;
    composed->component->link(
        linkedProgram.writeRef(),
        diagnosticsBlob.writeRef());
    return std::unique_ptr<SlangComponentOpaque>(new SlangComponentOpaque(linkedProgram, diagnosticsBlob));
}

std::unique_ptr<SlangComponentOpaque> SlangCompilerOpaque::link_module(std::unique_ptr<SlangModuleOpaque> module) const {
    Slang::ComPtr<slang::IComponentType> linkedProgram;
    Slang::ComPtr<slang::IBlob> diagnosticsBlob;
    module->module->link(
        linkedProgram.writeRef(),
        diagnosticsBlob.writeRef());
    return std::unique_ptr<SlangComponentOpaque>(new SlangComponentOpaque(linkedProgram, diagnosticsBlob));
}

std::unique_ptr<SlangComponentListOpaque> new_slang_component_list() {
    return std::unique_ptr<SlangComponentListOpaque>(new SlangComponentListOpaque());
}

std::unique_ptr<SlangCompilerOpaque> new_slang_compiler() {
  return std::unique_ptr<SlangCompilerOpaque>(new SlangCompilerOpaque());
}

// struct SlangEntryPointReflection {
//        name: String,
//        stage: ShaderStage,
//        param_reflections: Vec<SlangParamReflection>,
//    }
//
//    struct SlangProgramReflection {
//        // this also assumes uniforms are a single struct of primitives
//        // which is mostly fine
//        uniform_reflections: Vec<SlangParamReflection>,
//        entry_point_reflections: Vec<SlangEntryPointReflection>
//    }
//
//    // a fully general data desc recursive enum class is a bit annoying
//    struct SlangParamReflection {
//        name: String,
//        var_type: VarType
//    }

SlangProgramReflection SlangComponentOpaque::get_program_reflection() const {
    SlangProgramReflection ret;
    Slang::ComPtr<slang::IBlob> diagnostics;

    std::cout << "entered function" << std::endl;

    slang::ProgramLayout* programLayout = component->getLayout(0, diagnostics.writeRef());

    if (diagnostics != nullptr)
        {
            std::cout << (const char*)diagnostics->getBufferPointer() << std::endl;
        }

    std::cout << "got program layout" << std::endl;



    int entryPointCount = programLayout->getEntryPointCount();

    std::cout << "got entry point count of " << entryPointCount << std::endl;

    for (int i = 0; i < entryPointCount; ++i)
    {
        SlangEntryPointReflection entry_refl;
        slang::EntryPointReflection* entryPointLayout = programLayout->getEntryPointByIndex(i);
        SlangStage stage = entryPointLayout->getStage();
        ShaderStage ret_stage;
        switch (stage) {
            case SLANG_STAGE_VERTEX:
                ret_stage = ShaderStage::Vertex;
                break;
            case SLANG_STAGE_FRAGMENT:
                ret_stage = ShaderStage::Fragment;
                break;
            case SLANG_STAGE_COMPUTE:
                ret_stage = ShaderStage::Compute;
                break;
            default:
                ret_stage = ShaderStage::None;
        }
        entry_refl.stage = ret_stage;
        entry_refl.name = rust::String(entryPointLayout->getName());

        ret.entry_point_reflections.push_back(entry_refl);
    }

    return ret;
}