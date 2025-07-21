struct Uniforms
{
    float4x4 model;
    float4x4 view;
    float4x4 projection;
}

struct VertexInput
{
    float3	position;
    float3	color;
};

struct Fragment
{
    float4 color;
};

struct VertexOutput
{
    float3          color;
    float4          sv_position     : SV_Position;
};

[shader("vertex")]
VertexOutput vertex_main(VertexInput input)
{
    VertexOutput output;

    float3 position = assembledVertex.position;
    float3 color    = assembledVertex.color;

    output.color = color;
    output.sv_position = float4(position, 1.0);

    return output;
}

[shader("fragment")]
Fragment fragmentMain(
    CoarseVertex coarseVertex : CoarseVertex) : SV_Target
{
    float3 color = coarseVertex.color;

    Fragment output;
    output.color = float4(color, 1.0);
    return output;
}