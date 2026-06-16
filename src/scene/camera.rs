#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub(crate) position: [f32; 3],
    pub(crate) direction: [f32; 3],
}

impl Camera {
    pub fn new(z: f32) -> Self {
        Self {
            position: [0.0, 0.0, z],
            direction: [0.0, 0.0, -1.0],
        }
    }

    pub fn position(&mut self, position: [f32; 3]) -> &mut Self {
        self.position = position;
        self
    }

    pub fn direction(&mut self, direction: [f32; 3]) -> &mut Self {
        self.direction = direction;
        self
    }

    pub fn x(&self) -> f32 {
        self.position[0]
    }

    pub fn y(&self) -> f32 {
        self.position[1]
    }

    pub fn z(&self) -> f32 {
        self.position[2]
    }

    pub(crate) fn matrix(&self, aspect: f32) -> [[f32; 4]; 4] {
        let projection = perspective_matrix(45.0_f32.to_radians(), aspect.max(0.001), 0.1, 1000.0);
        let view = look_to_matrix(self.position, normalize(self.direction), [0.0, 1.0, 0.0]);
        multiply_matrix(projection, view)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(10.0)
    }
}

fn perspective_matrix(fov_y: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov_y * 0.5).tan();
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, near * far / (near - far), 0.0],
    ]
}

fn look_to_matrix(position: [f32; 3], direction: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let forward = normalize(direction);
    let right = normalize(cross(forward, up));
    let up = cross(right, forward);
    [
        [right[0], up[0], -forward[0], 0.0],
        [right[1], up[1], -forward[1], 0.0],
        [right[2], up[2], -forward[2], 0.0],
        [
            -dot(right, position),
            -dot(up, position),
            dot(forward, position),
            1.0,
        ],
    ]
}

fn multiply_matrix(left: [[f32; 4]; 4], right: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut matrix = [[0.0; 4]; 4];
    for column in 0..4 {
        for row in 0..4 {
            matrix[column][row] = left[0][row] * right[column][0]
                + left[1][row] * right[column][1]
                + left[2][row] * right[column][2]
                + left[3][row] * right[column][3];
        }
    }
    matrix
}

fn normalize(vector: [f32; 3]) -> [f32; 3] {
    let length = dot(vector, vector).sqrt();
    if length <= f32::EPSILON {
        return [0.0, 0.0, -1.0];
    }
    [vector[0] / length, vector[1] / length, vector[2] / length]
}

fn cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}
