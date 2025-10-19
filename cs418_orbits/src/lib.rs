use std::error::Error;
use std::f32::consts::PI;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, Weak};
use instant::Instant;
use poisson_renderer::{init_logger, run_game, shader, PoissonGame};
use console_error_panic_hook;
use poisson_renderer::input::Input;
use poisson_renderer::render_backend::{DrawletHandle, Mat4Ubo, PipelineHandle, RenderBackend, LayerHandle};
use poisson_renderer::render_backend::web::{CreateDrawletWgpu, WgpuBuffer, WgpuPipeline, WgpuRenderBackend};
use winit::keyboard::{KeyCode, PhysicalKey};
use cgmath as cg;
use cgmath::{relative_ne, Matrix4, SquareMatrix, Vector3, Zero};
use fs_embed::fs_embed;
use poisson_renderer::math::utils::{orthographic, perspective};
// #[cfg(not(target_arch = "wasm32"))]
// use poisson_renderer::render_backend::vulkan::{CreateDrawletVulkan, VulkanRenderBackend};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;
use poisson_renderer::render_backend::render_interface::{ColoredMesh, ColoredMeshData, ColoredVertex, WgpuMesh, TexturedMesh};
use poisson_renderer::render_backend::web::colored_mesh::{ColoredMeshDrawlet, ColoredMeshPipeline};
use rj::Own;

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run_wasm() {
    console_error_panic_hook::set_once();
    run().unwrap();
}

pub fn run() ->  Result<(), impl Error> {
    init_logger();
    run_game::<Orbits>()
}

struct CelestialBody {
    drawlet: rj::Own<ColoredMeshDrawlet>,
    base_position: cg::Vector3<f32>,
    transform: Matrix4<f32>,
    spin_speed: f32,
    spin_angle: f32,
    revolve_speed: f32,
    revolve_radius: f32,
    revolve_angle: f32,
    scale: f32,
    parent: rj::Ref<CelestialBody>,
    children: Vec<rj::Own<CelestialBody>>
}

impl CelestialBody {
    fn new(pipeline: &rj::Own<ColoredMeshPipeline>,
           mesh: &Arc<WgpuMesh>,
           spin_speed: f32,
           revolve_radius: f32,
           revolve_speed: f32,
           scale: f32
    ) -> Self
    {
        let body_data = ColoredMeshData {
            mvp_data: Matrix4::identity(),
            mesh: mesh.clone(),
        };
        let drawlet = pipeline.access().create_drawlet(body_data);

        CelestialBody {
            drawlet,
            base_position: cg::Vector3 {x: 0f32, y: 0f32, z: 0f32 },
            transform: Matrix4::identity(),
            spin_speed,
            spin_angle: 0.0,
            revolve_speed,
            revolve_radius,
            revolve_angle: 0.0,
            parent: rj::Ref::null(),
            children: Vec::new(),
            scale,
        }
    }

    fn set_mvp(self: &mut Self, renderer: &mut WgpuRenderBackend, mvp: cgmath::Matrix4<f32>) {
        self.drawlet.access().set_mvp(Mat4Ubo{ data: mvp  })
    }

    pub fn update(&mut self, renderer: &mut WgpuRenderBackend, view_proj: cgmath::Matrix4<f32>, dt: f32) {
        self.spin_angle += dt * self.spin_speed;
        self.revolve_angle += dt * self.revolve_speed;

        let rotation = Matrix4::<f32>::from_angle_y(cg::Rad(self.spin_angle));
        let translation = Vector3 {
            x: self.revolve_radius *  self.revolve_angle.sin(),
            y: 0f32,
            z: self.revolve_radius *  self.revolve_angle.cos()
        };
        let scale = Matrix4::<f32>::from_scale(self.scale);

        let tform =  Matrix4::from_translation(self.base_position + translation)* rotation * scale;

        self.set_mvp(renderer, view_proj * tform);

        for c in &mut self.children {
            c.access().base_position = self.base_position + translation;
            c.access().update(renderer, view_proj, dt);
        }
    }

    pub fn add_child(self: &mut Self, celestial_body: rj::Own<CelestialBody>) {
        self.children.push(celestial_body);
    }

    pub fn add_child_own(own: &Own<CelestialBody>, child: Own<CelestialBody>) {
        child.access().parent = own.borrow();
        own.access().children.push(child);
    }

}

pub struct Orbits {
    scene_render_pass: Option<LayerHandle>,
    colored_mesh_pipeline: Option<PipelineHandle<ColoredMesh>>,
    sun: Option<CelestialBody>,
    last_time: Instant,
    elapsed_time: f32,
    assets: fs_embed::Dir,
}

impl PoissonGame for Orbits {
    type Ren = WgpuRenderBackend;

    fn new() -> Self {
        static FILES: fs_embed::Dir = fs_embed!("assets");

        Self {
            scene_render_pass: None,
            colored_mesh_pipeline: None,
            sun: None,
            last_time: Instant::now(),
            elapsed_time: 0f32,
            assets: FILES.clone().auto_dynamic()
        }
    }

    fn pre_init(self: &mut Self, input: &mut Input) {
        //input.set_mapping("up", vec![PhysicalKey::Code(KeyCode::KeyW)]);
    }

    fn init(self: &mut Self, _input: &mut Input, renderer: &mut Self::Ren) {
        self.last_time = Instant::now();
        let tetrahedron_indices = [0u32, 1, 2, 0, 2, 3, 0, 3, 1, 1, 2, 3];
        let tetrahedron_vertices = vec![
            ColoredVertex {pos: [ 1f32,  1f32,  1f32], color: [1f32, 1f32, 1f32]},
            ColoredVertex {pos: [-1f32, -1f32,  1f32], color: [0f32, 0f32, 1f32]},
            ColoredVertex {pos: [-1f32,  1f32, -1f32], color: [0f32, 1f32, 0f32]},
            ColoredVertex {pos: [ 1f32, -1f32, -1f32], color: [1f32, 0f32, 0f32]}
        ];
        let tetrahedron_mesh = Arc::new(WgpuMesh {
            index: renderer.create_index_buffer(&tetrahedron_indices),
            vertex: renderer.create_vertex_buffer(tetrahedron_vertices.as_slice())
        });
        
        let octahedron_indices = [
            [0u32,1,2], [0,2,3], [0,3,4], [0,4,1], 
            [5,1,4], [5,4,3], [5,3,2], [5,2,1]
        ].concat();
        let octahedron_vertices = vec![
            ColoredVertex {pos: [ 1f32,  0f32,  0f32], color: [  1f32, 0.5f32, 0.5f32]},
            ColoredVertex {pos: [ 0f32,  1f32,  0f32], color: [0.5f32,   1f32, 0.5f32]},
            ColoredVertex {pos: [ 0f32,  0f32,  1f32], color: [0.5f32, 0.5f32,   1f32]},
            ColoredVertex {pos: [ 0f32, -1f32,  0f32], color: [0.5f32,   0f32, 0.5f32]},
            ColoredVertex {pos: [ 0f32,  0f32, -1f32], color: [0.5f32, 0.5f32,   0f32]},
            ColoredVertex {pos: [-1f32,  0f32,  0f32], color: [  0f32, 0.5f32, 0.5f32]}
        ];
        let octahedron_mesh = Arc::new(WgpuMesh {
            index: renderer.create_index_buffer(octahedron_indices.as_slice()),
            vertex: renderer.create_vertex_buffer(octahedron_vertices.as_slice())
        });

        let triangle_shader = self.assets.get_file(shader!("shaders/colored_mesh")).unwrap();
        let triangle_shader_content = triangle_shader.read_str().unwrap();

        let r_handle = renderer.create_render_pass();
        let p_handle =
            r_handle.access().create_pipeline::<ColoredMesh>(
                "cs418_logo/assets/shaders/colored_mesh",
                triangle_shader_content.as_str());

        self.sun = Some(CelestialBody::new(
            &p_handle, &octahedron_mesh,
            PI, 0f32, 0f32, 1f32
        ));

        let earth = rj::Own::new(CelestialBody::new(
                 &p_handle, &octahedron_mesh,
                4f32*PI, 2f32, 0.2f32*PI, 0.2f32
        ));
        let moon = rj::Own::new(
            CelestialBody::new(
                 &p_handle, &tetrahedron_mesh,
                2f32*PI, 0.5f32, 2f32*PI, 0.08f32
            ));
        earth.access().add_child(moon);


        let mars = rj::Own::new(
            CelestialBody::new(
                &p_handle, &octahedron_mesh,
                4f32/2.2f32 * PI, 2f32 * 1.6f32, 0.2f32 / 1.9f32 *PI, 0.2f32 * 0.9f32
            )
        );

        let phobos = rj::Own::new(
            CelestialBody::new(
                &p_handle, &tetrahedron_mesh,
                6f32 * PI, 0.4f32, 4f32 * PI, 0.1f32));

        let deimos = rj::Own::new(
            CelestialBody::new(&p_handle, &tetrahedron_mesh,
                4f32/2.0f32 * PI, 0.8f32, 4f32/2.0f32 * PI, 0.05f32)
        );


        mars.access().add_child(phobos);
        
        CelestialBody::add_child_own(&mars, deimos);
        // mars.deref().as_mut().unwrap().add_child(deimos);

        
        self.sun.as_mut().unwrap().add_child(earth);
        self.sun.as_mut().unwrap().add_child(mars);

    }

    fn update(self: &mut Self, input: &mut Input, renderer: &mut Self::Ren) {
        let delta_time = self.last_time.elapsed().as_secs_f32();
        self.last_time = Instant::now();

        self.elapsed_time += delta_time;

        let v = cgmath::Matrix4::look_at_rh(
            cgmath::Point3::new(0.0, 2.0, 8.0),
            cgmath::Point3::new(0.0, 0.0, 0.0),
            cgmath::Vector3::new(0.0, 1.0, 0.0));
        let aspect_ratio = (renderer.get_width() as f32)/(renderer.get_height() as f32);

        let p = perspective(PI/12f32, aspect_ratio, 0.1, 100.0, Self::Ren::PERSPECTIVE_ALIGNMENT);

        self.sun.as_mut().unwrap().update(renderer, p * v, delta_time);

    }
}
