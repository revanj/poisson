Custom game engine for future graphics needs. (Heavily WIP)

Supports compilation to desktop (`VulkanRenderBackend` and `WgpuRenderBackend`) or Web (`WgpuRenderBackend` Only, via `wasm-pack`).

Shader language in Slang, which gets compiled into WGSL or SPIR-V depending on the render backend. Slang stuff is in the slang_refl folder. Currently, the reflection utils can list all the unnested uniforms and varyings on a shader.

For a tiny sample project that uses the engine, see `nothing_game` folder.
