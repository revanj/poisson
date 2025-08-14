struct Uniform
{
    float4x4 mvp;
}

struct TextureVar {
    Texture2D<float4> texture;
    SamplerState sampler;
}

ParameterBlock<Uniform> uniform_var;
ParameterBlock<TextureVar> texture_var;


struct AssembledVertex
{
    float3 position : POSITION;
    float2 tex_coord: TEX_COORD;
};

struct CoarseVertex
{
    float2 tex_coord;
};

struct Fragment
{
    float4 color;
};

struct VertexStageOutput
{
    CoarseVertex    coarseVertex    : CoarseVertex;
    float4          sv_position     : SV_Position;
};

[shader("vertex")]
VertexStageOutput vertex(
    AssembledVertex assembledVertex)
{
    VertexStageOutput output;

    float4 position = mul(float4(assembledVertex.position, 1.0), uniform_var.mvp);

    output.coarseVertex.tex_coord = assembledVertex.tex_coord;
    output.sv_position = position;
    // output.sv_position = mul(modelViewProjection, float4(position, 1.0));

    return output;
}

[shader("fragment")]
Fragment fragment(
    CoarseVertex coarseVertex : CoarseVertex) : SV_Target
{
    Fragment output;
    output.color = texture_var.texture.Sample(texture_var.sampler, coarseVertex.tex_coord);
    output.color = texture_var.texture.Sample(texture_var.sampler, coarseVertex.tex_coord);
    return output;
}