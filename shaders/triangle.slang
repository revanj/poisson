struct Uniform
{
    float4x4 model;
    float4x4 view;
    float4x4 projection;
}

[[vk::binding(0)]] Uniform uniform_var;
[[vk::binding(1)]] Sampler2D sampler;


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
VertexStageOutput vertexMain(
    AssembledVertex assembledVertex)
{
    VertexStageOutput output;

    float4 position = mul(float4(assembledVertex.position, 1.0), mul(uniform_var.model, mul(uniform_var.view, uniform_var.projection)));

    output.coarseVertex.tex_coord = assembledVertex.tex_coord;
    output.sv_position = position;
    // output.sv_position = mul(modelViewProjection, float4(position, 1.0));

    return output;
}

[shader("fragment")]
Fragment fragmentMain(
    CoarseVertex coarseVertex : CoarseVertex) : SV_Target
{
    Fragment output;
    output.color = sampler.Sample(coarseVertex.tex_coord);
    return output;
}