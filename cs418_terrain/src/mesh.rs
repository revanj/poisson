use cgmath::num_traits::FloatConst;
use rand::Rng;
use poisson_renderer::render_backend::render_interface::drawlets::lit_colored_mesh::NormalColoredVertex;

pub fn mesh_grid(n_segments: usize, n_faults: usize) -> (Vec<NormalColoredVertex>, Vec<u32>) {
    let n_vertices = n_segments + 1;
    let n_segments_f = n_segments as f32;
    let mut vertices = Vec::with_capacity(n_vertices * n_vertices);

    // lay out the vertices
    for i in 0..n_vertices {
        for j in 0..n_vertices {
            let f_i = i as f32;
            let f_j = j as f32;
            let (pt_x, pt_z) = (
                (f_i / n_segments_f -0.5f32) * 2f32,
                (f_j / n_segments_f -0.5f32) * 2f32
            );
            let vertex = NormalColoredVertex {
                pos: [pt_x, 0f32, pt_z],
                color: [0.8f32, 0.8f32, 0.8f32],
                normal: [0f32, 0f32, 0f32],
            };

            vertices.push(vertex);
        }
    }

    let mut rng = rand::rng();

    for _ in 0..n_faults {
        let rand_x: f32 = rng.random::<f32>() * 2f32 - 1f32;
        let rand_z: f32 = rng.random::<f32>() * 2f32 - 1f32;
        let p = rj::Vector::<f32, 2>::new([rand_x, rand_z]);
        let rand_theta: f32 = rng.random::<f32>() * f32::PI() * 2f32;
        let n = rj::Vector::<f32, 2>::new([rand_theta.cos(), rand_theta.sin()]);

        for pt in vertices.iter_mut() {
            let b = rj::Vector::<f32, 2>::new([pt.pos[0], pt.pos[2]]);
            if (b - p) * n > 0f32 {
                pt.pos[1] += 0.1;
            } else {
                pt.pos[1] -= 0.1;
            }
        }
    }

    let max_height = vertices.iter().map(|v| v.pos[1]).reduce(|max, y| if y > max { y } else { max }).unwrap();
    let min_height = vertices.iter().map(|v| v.pos[1]).reduce(|min, y| if y < min { y } else { min }).unwrap();

    if max_height > min_height {
        for pt in vertices.iter_mut() {
            pt.pos[1] = (pt.pos[1] - min_height) / (max_height - min_height);
        }
    } else {
        for pt in vertices.iter_mut() {
            pt.pos[1] = 0f32;
        }
    }




    let mut indices = Vec::new();
    for i in 0..n_segments {
        for j in 0..n_segments {
            let first_vertex  = ( i      * n_vertices + j    ) as u32;
            let second_vertex = ( i      * n_vertices + j + 1) as u32;
            let third_vertex  = ((i + 1) * n_vertices + j    ) as u32;
            let fourth_vertex = ((i + 1) * n_vertices + j + 1) as u32;

            indices.push(first_vertex);
            indices.push(second_vertex);
            indices.push(third_vertex);

            indices.push(second_vertex);
            indices.push(fourth_vertex);
            indices.push(third_vertex);
        }
    }

    for vert_index in 0..vertices.len() {
        let mut total_cross_product = rj::Vector::<f32, 3>::new([0f32; 3]);
        for occurrence_index in 0..indices.len() {
            if indices[occurrence_index] == vert_index as u32 {
                let offset = occurrence_index - occurrence_index % 3;
                let first_index = indices[offset];
                let second_index = indices[offset + 1];
                let third_index = indices[offset + 2];

                let first_vertex_pos = rj::Vector::new(vertices[first_index as usize].pos);
                let second_vertex_pos = rj::Vector::new(vertices[second_index as usize].pos);
                let third_vertex_pos = rj::Vector::new(vertices[third_index as usize].pos);


                let first_vector = second_vertex_pos - first_vertex_pos;
                let second_vector = third_vertex_pos - first_vertex_pos;


                total_cross_product += first_vector.cross(second_vector);

                // println!("total cross_product is {:?}", total_cross_product.data);

            }
        }
        vertices[vert_index].normal = total_cross_product.normalized().data;
        println!("normal is {:?}", vertices[vert_index].normal)
    }

    (vertices, indices)
}