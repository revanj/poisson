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

    slang::ProgramLayout* programLayout = component->getLayout(0, diagnostics.writeRef());
    auto uniform_struct_layout = programLayout->getGlobalParamsVarLayout()->getTypeLayout();

//    // found the uniforms
//    if (uniform_struct_layout->getKind() == slang::TypeReflection::Kind::Struct) {
//        int paramCount = uniform_struct_layout->getFieldCount();
//
//        entry_refl.misc_reflections.name = rust::String("misc_params");
//
//        for (int j = 0; j < paramCount; j++)
//        {
//            auto param = input_struct_layout->getFieldByIndex(j);
//            auto param_type_layout = param->getTypeLayout();
//
//            auto param_kind = param_type_layout->getKind(); // slang::TypeReflection::Kind::
//            auto param_type = param_type_layout->getType(); // slang::TypeReflection*
//
//            if (param_kind != slang::TypeReflection::Kind::Struct) {
//                std::cout << "found misc param" << std::endl;
//                // put it into misc
//                continue;
//            }
//
//            SlangStructReflection struct_param;
//            struct_param.name = rust::String(param->getName());
//            auto field_count = param_type_layout->getFieldCount();
//            for (int k = 0; k < field_count; k++) {
//
//                auto field = param_type_layout->getFieldByIndex(k);
//
//                auto field_type_layout = field->getTypeLayout();
//                auto field_kind = field_type_layout->getKind(); // slang::TypeReflection::Kind::
//
//                SlangFieldReflection field_refl;
//                field_refl.name = rust::String(field->getName());
//                field_refl.var_type = VarType::Undefined;
//
//                if (field_kind == slang::TypeReflection::Kind::Struct) {
//                    std::cout << "found overly nested struct!" << std::endl;
//                    continue;
//                }
//
//                auto field_type = field_type_layout->getType();
//
//                switch (field_kind) {
//                    case slang::TypeReflection::Kind::Vector:
//                        auto vec_length = field_type->getElementCount();
//
//                        auto element_scalar_type = field_type->getElementType()->getScalarType();
//
//                        if (element_scalar_type == slang::TypeReflection::ScalarType::Float32) {
//                            if (vec_length == 2) {
//                                field_refl.var_type = VarType::Float2;
//                            } else if (vec_length == 3) {
//                                field_refl.var_type = VarType::Float3;
//                            } else if (vec_length == 4) {
//                                field_refl.var_type = VarType::Float4;
//                            }
//                        }
//                        break;
//                }
//
//                struct_param.fields.push_back(field_refl);
//            }
//
//            entry_refl.struct_reflections.push_back(struct_param);
//        }
//    }



    if (diagnostics != nullptr)
    {
        std::cout << (const char*)diagnostics->getBufferPointer() << std::endl;
    }

    int entryPointCount = programLayout->getEntryPointCount();

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

        auto input_struct_layout = entryPointLayout->getVarLayout()->getTypeLayout();

        // this is the normal case that we can handle
        if (input_struct_layout->getKind() == slang::TypeReflection::Kind::Struct) {
            int paramCount = input_struct_layout->getFieldCount();

            entry_refl.misc_reflections.name = rust::String("misc_params");

            for (int j = 0; j < paramCount; j++)
            {
                auto param = input_struct_layout->getFieldByIndex(j);
                auto param_type_layout = param->getTypeLayout();

                auto param_kind = param_type_layout->getKind(); // slang::TypeReflection::Kind::
                auto param_type = param_type_layout->getType(); // slang::TypeReflection*
                auto param_layout_unit = param_type_layout->getCategoryByIndex(0);
                auto param_offset = param->getOffset(param_layout_unit);

                if (param_kind != slang::TypeReflection::Kind::Struct) {
                    std::cout << "found misc param" << std::endl;
                    // put it into misc
                    continue;
                }

                SlangStructReflection struct_param;
                struct_param.name = rust::String(param->getName());
                struct_param.binding = j;
                auto field_count = param_type_layout->getFieldCount();
                for (int k = 0; k < field_count; k++) {

                    auto field = param_type_layout->getFieldByIndex(k);

                    auto field_type_layout = field->getTypeLayout();

                    auto field_layout_unit = field_type_layout->getCategoryByIndex(0);

                    auto field_offset = field->getOffset(field_layout_unit);

                    auto field_kind = field_type_layout->getKind(); // slang::TypeReflection::Kind::

                    SlangFieldReflection field_refl;
                    field_refl.name = rust::String(field->getName());
                    field_refl.location = field_offset + param_offset;
                    field_refl.var_type = VarType::Undefined;

                    if (field_kind == slang::TypeReflection::Kind::Struct) {
                        std::cout << "found overly nested struct!" << std::endl;
                        continue;
                    }

                    auto field_type = field_type_layout->getType();

                    switch (field_kind) {
                        case slang::TypeReflection::Kind::Vector:
                            auto vec_length = field_type->getElementCount();

                            auto element_scalar_type = field_type->getElementType()->getScalarType();

                            if (element_scalar_type == slang::TypeReflection::ScalarType::Float32) {
                                if (vec_length == 2) {
                                    field_refl.var_type = VarType::Float2;
                                } else if (vec_length == 3) {
                                    field_refl.var_type = VarType::Float3;
                                } else if (vec_length == 4) {
                                    field_refl.var_type = VarType::Float4;
                                }
                            }
                            break;
                    }

                    struct_param.fields.push_back(field_refl);
                }

                entry_refl.struct_reflections.push_back(struct_param);
            }
        }

        ret.entry_point_reflections.push_back(entry_refl);
    }



    return ret;
}