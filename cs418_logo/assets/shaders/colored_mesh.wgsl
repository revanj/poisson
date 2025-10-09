struct _MatrixStorage_float4x4std140_0
{
    @align(16) data_0 : array<vec4<f32>, i32(4)>,
};

struct Uniform_std140_0
{
    @align(16) mvp_0 : _MatrixStorage_float4x4std140_0,
};

@binding(0) @group(0) var<uniform> uniform_var_0 : Uniform_std140_0;
struct VertexStageOutput_0
{
    @location(0) color_0 : vec3<f32>,
    @builtin(position) sv_position_0 : vec4<f32>,
};

struct vertexInput_0
{
    @location(0) position_0 : vec3<f32>,
    @location(1) color_1 : vec3<f32>,
};

struct CoarseVertex_0
{
     _S1 : vec3<f32>,
};

struct VertexStageOutput_1
{
     coarseVertex_0 : CoarseVertex_0,
     _S2 : vec4<f32>,
};

@vertex
fn vertex( _S3 : vertexInput_0) -> VertexStageOutput_0
{
    var position_1 : vec4<f32> = (((mat4x4<f32>(uniform_var_0.mvp_0.data_0[i32(0)][i32(0)], uniform_var_0.mvp_0.data_0[i32(0)][i32(1)], uniform_var_0.mvp_0.data_0[i32(0)][i32(2)], uniform_var_0.mvp_0.data_0[i32(0)][i32(3)], uniform_var_0.mvp_0.data_0[i32(1)][i32(0)], uniform_var_0.mvp_0.data_0[i32(1)][i32(1)], uniform_var_0.mvp_0.data_0[i32(1)][i32(2)], uniform_var_0.mvp_0.data_0[i32(1)][i32(3)], uniform_var_0.mvp_0.data_0[i32(2)][i32(0)], uniform_var_0.mvp_0.data_0[i32(2)][i32(1)], uniform_var_0.mvp_0.data_0[i32(2)][i32(2)], uniform_var_0.mvp_0.data_0[i32(2)][i32(3)], uniform_var_0.mvp_0.data_0[i32(3)][i32(0)], uniform_var_0.mvp_0.data_0[i32(3)][i32(1)], uniform_var_0.mvp_0.data_0[i32(3)][i32(2)], uniform_var_0.mvp_0.data_0[i32(3)][i32(3)])) * (vec4<f32>(_S3.position_0, 1.0f))));
    var output_0 : VertexStageOutput_1;
    output_0.coarseVertex_0._S1 = _S3.color_1;
    output_0._S2 = position_1;
    var _S4 : VertexStageOutput_0;
    _S4.color_0 = output_0.coarseVertex_0._S1;
    _S4.sv_position_0 = output_0._S2;
    return _S4;
}

struct Fragment_0
{
    @location(0) color_2 : vec4<f32>,
};

struct pixelInput_0
{
    @location(0) _S5 : vec3<f32>,
};

struct pixelInput_1
{
     coarseVertex_1 : CoarseVertex_0,
};

@fragment
fn fragment( _S6 : pixelInput_0) -> Fragment_0
{
    var _S7 : pixelInput_1;
    _S7.coarseVertex_1._S1 = _S6._S5;
    var output_1 : Fragment_0;
    output_1.color_2 = vec4<f32>(_S7.coarseVertex_1._S1, 1.0f);
    return output_1;
}

