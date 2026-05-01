#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub(crate) position: [f32; 3],
}

impl Camera {
    pub fn new(z: f32) -> Self {
        Self {
            position: [0.0, 0.0, z],
        }
    }

    pub fn position(&mut self, position: [f32; 3]) -> &mut Self {
        self.position = position;
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
        let z = self.position[2].max(1.0);
        let x_scale = z / aspect.max(0.001);
        [
            [x_scale, 0.0, 0.0, 0.0],
            [0.0, z, 0.0, 0.0],
            [0.0, 0.0, -0.5, -1.0],
            [
                -self.position[0] * x_scale,
                -self.position[1] * z,
                z * 0.5,
                z,
            ],
        ]
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(64.0)
    }
}
