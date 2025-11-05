fn mesh_grid(n_segments: usize) -> (Vec<f32>, Vec<u32>) {
    let n_vertices = n_segments + 1;
    let n_segments_f = n_segments as f32;
    let mut vertices = Vec::with_capacity(n_vertices * n_vertices);
    for i in 0..n_vertices {
        for j in 0..n_vertices {
            let f_i = i as f32;
            let f_j = j as f32;
            let (pt_x, pt_z) = (
                (f_i / n_segments_f -0.5f32) * 2f32,
                (f_j / n_segments_f -0.5f32) * 2f32
            );

            vertices.push(pt_x);
            vertices.push(0f32);
            vertices.push(pt_z);
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

    (vertices, indices)
}