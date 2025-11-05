struct Uniform
{
    float4x4 mvp;
}

struct Light
{
    float4 light_dir;
}

ParameterBlock<Uniform> uniform_var;
ParameterBlock<Light> light_var;


struct AssembledVertex
{
    float3 position : POSITION;
    float3 color: COLOR;
    float3 normal: NORMAL;
};

struct CoarseVertex
{
    float3 color;
    float3 normal;
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

    output.coarseVertex.color = assembledVertex.color;
    output.coarseVertex.normal = assembledVertex.normal;

    output.sv_position = position;

    return output;
}

[shader("fragment")]
Fragment fragment(CoarseVertex coarseVertex : CoarseVertex) : SV_Target
{
    Fragment output;
    output.color = float4(light_var.light_dir.xyz, 1.0);
    return output;
}