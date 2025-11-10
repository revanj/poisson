struct Uniform
{
    float4x4 mvp;
}

struct Light
{
    float4 light_dir;
}

struct View
{
    float4 view_dir;
}

ParameterBlock<Uniform> uniform_var;
ParameterBlock<Light> light_var;
ParameterBlock<View> view_var;


struct AssembledVertex
{
    float3 position : POSITION;
    float4 color: COLOR;
    float3 normal: NORMAL;
};

struct CoarseVertex
{
    float4 color;
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
    float alpha = coarseVertex.color.w;
    float3 light_vec = normalize(light_var.light_dir.xyz);
    float3 view_vec = normalize(view_var.view_dir.xyz);
    float3 normal_vec = normalize(coarseVertex.normal);
    float3 ambient = float3(0.1, 0.1, 0.1);
    float3 diffuse = max(dot(light_vec, normal_vec), 0.0) * coarseVertex.color.xyz * (1.0 - alpha);
    float3 reflect_vector = (2 * dot(normal_vec, light_vec)) * normal_vec - light_vec;
    float3 rv = max(dot(reflect_vector, view_vec), 0);

    float3 specular = 3.0 * alpha * pow(rv, 50);

    output.color = float4(float3(alpha), 1.0);
    return output;
}