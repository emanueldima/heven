const SPREAD: f32 = 8.0;

pub(crate) fn sdf(mask: &[u8], size: [usize; 2]) -> Vec<u8> {
    let radius = SPREAD.ceil() as i32;
    let mut sdf = vec![0; size[0] * size[1]];
    for y in 0..size[1] {
        for x in 0..size[0] {
            let index = y * size[0] + x;
            let inside = mask[index] > 127;
            let mut nearest = SPREAD * SPREAD;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let neighbor = [x as i32 + dx, y as i32 + dy];
                    if neighbor[0] < 0
                        || neighbor[1] < 0
                        || neighbor[0] >= size[0] as i32
                        || neighbor[1] >= size[1] as i32
                    {
                        continue;
                    }
                    let neighbor_index = neighbor[1] as usize * size[0] + neighbor[0] as usize;
                    if (mask[neighbor_index] > 127) == inside {
                        continue;
                    }
                    nearest = nearest.min((dx * dx + dy * dy) as f32);
                }
            }
            let alpha = mask[index] as f32 / 255.0;
            let signed_distance = if alpha > 0.0 && alpha < 1.0 {
                alpha - 0.5
            } else {
                nearest.sqrt() * if inside { 1.0 } else { -1.0 }
            };
            sdf[index] =
                ((0.5 + signed_distance / (SPREAD * 2.0)).clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    sdf
}
