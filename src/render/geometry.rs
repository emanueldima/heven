#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    position: [f32; 3],
    color: [u8; 4],
    tex_coord: [f32; 2],
}

pub(crate) const QUAD_VERTEX_COUNT: usize = 6;

pub(crate) fn push_quad(
    vertices: &mut Vec<Vertex>,
    top_left: [f32; 3],
    size: [f32; 2],
    color: [u8; 4],
    tex_coords: [[f32; 2]; 4],
) {
    let corners = [
        [top_left[0], top_left[1] - size[1]],
        [top_left[0] + size[0], top_left[1] - size[1]],
        [top_left[0] + size[0], top_left[1]],
        [top_left[0], top_left[1]],
    ];
    for index in [0, 1, 2, 0, 2, 3] {
        vertices.push(Vertex {
            position: [corners[index][0], corners[index][1], top_left[2]],
            color,
            tex_coord: tex_coords[index],
        });
    }
}
