struct Uniform
{
    float4x4 model;
    float4x4 view;
    float4x4 projection;
}

struct AssembledVertex
{
    float3	position : POSITION;
    float3	color    : COLOR;
};

struct CoarseVertex
{
    float3 color;
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
    ParameterBlock<Uniform> uniform_var,
    AssembledVertex assembledVertex)
{
    VertexStageOutput output;

    // gl_Position = ubo.proj * ubo.view * ubo.model * vec4(inPosition, 0.0, 1.0);
    float4 position = mul(float4(assembledVertex.position, 1.0), mul(uniform_var.model, mul(uniform_var.view, uniform_var.projection)));
    float3 color    = assembledVertex.color;

    output.coarseVertex.color = color;
    output.sv_position = position;
    // output.sv_position = mul(modelViewProjection, float4(position, 1.0));

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