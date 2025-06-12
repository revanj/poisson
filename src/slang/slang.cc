#include "rust-renderer/src/slang/slang.h"

#include "rust-renderer/external/slang/include/slang.h"
#include "rust-renderer/external/slang/include/slang-com-ptr.h"
#include "rust-renderer/external/slang/include/slang-com-helper.h"

#include <iostream>
#include <array>

const char* shortestShader =
"RWStructuredBuffer<float> result;"
"[shader(\"compute\")]"
"[numthreads(1,1,1)]"
"void computeMain(uint3 threadId : SV_DispatchThreadID)"
"{"
"    result[threadId.x] = threadId.x;"
"}";

SlangEntryPointOpaque::SlangEntryPointOpaque(
    Slang::ComPtr<slang::IEntryPoint> entry):
        entry_point(entry) {}

SlangModuleOpaque::SlangModuleOpaque(Slang::ComPtr<slang::IModule> mod, Slang::ComPtr<slang::IBlob> blob)
    : module(mod), diagnostics_blob(blob) {}

std::unique_ptr<SlangEntryPointOpaque> SlangModuleOpaque::find_entry_point_by_name(rust::Str name) const {
     Slang::ComPtr<slang::IEntryPoint> entry;
     module->findEntryPointByName(((std::string)name).c_str(), entry.writeRef());

     if (!entry)
     {
        std::cout << "no entry point!" << std::endl;
     }

     return std::unique_ptr<SlangEntryPointOpaque>(new SlangEntryPointOpaque(entry));
}

void SlangComponentListOpaque::add_module(std::unique_ptr<SlangModuleOpaque> module) {
    components.push_back(module->module);
}

void SlangComponentListOpaque::add_entry_point(std::unique_ptr<SlangEntryPointOpaque> entry_point) {
    components.push_back(entry_point->entry_point);
}

SlangComponentOpaque::SlangComponentOpaque(Slang::ComPtr<slang::IComponentType> comp, Slang::ComPtr<slang::IBlob> blob):
    component(comp), diagnostics_blob(blob) {}

SlangCompilerOpaque::SlangCompilerOpaque() {
    createGlobalSession(globalSession.writeRef());
    slang::SessionDesc sessionDesc = {};
    slang::TargetDesc targetDesc = {};
    targetDesc.format = SLANG_SPIRV;
    targetDesc.profile = globalSession->findProfile("spirv_1_5");

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





//void diagnoseIfNeeded(slang::IBlob* diagnosticsBlob)
//{
//
//}


//int compile() {
//    Slang::ComPtr<slang::IModule> slangModule;
//    {
//        Slang::ComPtr<slang::IBlob> diagnosticsBlob;
//        slangModule = session->loadModuleFromSourceString(
//            "shortest",                  // Module name
//            "shortest.slang",            // Module path
//            shortestShader,              // Shader source code
//            diagnosticsBlob.writeRef()); // Optional diagnostic container
//        diagnoseIfNeeded(diagnosticsBlob);
//        if (!slangModule)
//        {
//            return -1;
//        }
//    }
//
//    // 4. Query Entry Points
//    Slang::ComPtr<slang::IEntryPoint> entryPoint;
//    {
//        Slang::ComPtr<slang::IBlob> diagnosticsBlob;
//        slangModule->findEntryPointByName("computeMain", entryPoint.writeRef());
//        if (!entryPoint)
//        {
//            std::cout << "Error getting entry point" << std::endl;
//            return -1;
//        }
//    }
//
//    // 5. Compose Modules + Entry Points
//    std::array<slang::IComponentType*, 2> componentTypes =
//        {
//            slangModule,
//            entryPoint
//        };
//
//    Slang::ComPtr<slang::IComponentType> composedProgram;
//    {
//        Slang::ComPtr<slang::IBlob> diagnosticsBlob;
//        SlangResult result = session->createCompositeComponentType(
//            componentTypes.data(),
//            componentTypes.size(),
//            composedProgram.writeRef(),
//            diagnosticsBlob.writeRef());
//        diagnoseIfNeeded(diagnosticsBlob);
//        SLANG_RETURN_ON_FAIL(result);
//    }
//
//    // 6. Link
//    Slang::ComPtr<slang::IComponentType> linkedProgram;
//    {
//        Slang::ComPtr<slang::IBlob> diagnosticsBlob;
//        SlangResult result = composedProgram->link(
//            linkedProgram.writeRef(),
//            diagnosticsBlob.writeRef());
//        diagnoseIfNeeded(diagnosticsBlob);
//        SLANG_RETURN_ON_FAIL(result);
//    }
//
//    // 7. Get Target Kernel Code
//    Slang::ComPtr<slang::IBlob> spirvCode;
//    {
//        Slang::ComPtr<slang::IBlob> diagnosticsBlob;
//        SlangResult result = linkedProgram->getEntryPointCode(
//            0,
//            0,
//            spirvCode.writeRef(),
//            diagnosticsBlob.writeRef());
//        diagnoseIfNeeded(diagnosticsBlob);
//        SLANG_RETURN_ON_FAIL(result);
//    }
//
//    std::cout << "Compiled " << spirvCode->getBufferSize() << " bytes of SPIR-V" << std::endl;
//    return 0;
//}

//std::unique_ptr<BlobstoreClient> new_blobstore_client() {
//  return std::unique_ptr<BlobstoreClient>(new BlobstoreClient());
//}