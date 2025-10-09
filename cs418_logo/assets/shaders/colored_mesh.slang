struct Uniform
{
    float4x4 mvp;
}

ParameterBlock<Uniform> uniform_var;


struct AssembledVertex
{
    float3 position : POSITION;
    float3 color: COLOR;
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
VertexStageOutput vertex(
    AssembledVertex assembledVertex)
{
    VertexStageOutput output;

    float4 position = mul(float4(assembledVertex.position, 1.0), uniform_var.mvp);

    output.coarseVertex.color = assembledVertex.color;
    output.sv_position = position;
    // output.sv_position = mul(modelViewProjection, float4(position, 1.0));

    return output;
}

[shader("fragment")]
Fragment fragment(
    CoarseVertex coarseVertex : CoarseVertex) : SV_Target
{
    Fragment output;
    output.color = float4(coarseVertex.color, 1.0);
    //output.color = float4(pow(coarseVertex.color, float3(2.2)), 1.0);
    return output;
}