#[derive(Clone, Copy, Debug)]
pub struct LinearRGB(pub(crate) u8, pub(crate) u8, pub(crate) u8, pub(crate) u8);

impl LinearRGB {
    pub fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self(red, green, blue, alpha)
    }

    pub(crate) fn as_bytes(self) -> [u8; 4] {
        [self.0, self.1, self.2, self.3]
    }

    pub(crate) fn as_floats(self) -> [f32; 4] {
        [
            self.0 as f32 / 255.0,
            self.1 as f32 / 255.0,
            self.2 as f32 / 255.0,
            self.3 as f32 / 255.0,
        ]
    }
}

pub fn rgb(red: u8, green: u8, blue: u8) -> LinearRGB {
    rgba(red, green, blue, 255)
}

pub fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> LinearRGB {
    LinearRGB::new(red, green, blue, alpha)
}

pub fn oklch(lightness: f32, chroma: f32, hue: f32) -> LinearRGB {
    let lightness = lightness.clamp(0.0, 1.0);
    let chroma = chroma.max(0.0);
    let hue = hue.rem_euclid(360.0).to_radians();
    let a = chroma * hue.cos();
    let b = chroma * hue.sin();
    let l = (lightness + 0.396_337_78 * a + 0.215_803_76 * b).powi(3);
    let m = (lightness - 0.105_561_346 * a - 0.063_854_17 * b).powi(3);
    let s = (lightness - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);

    rgb(
        srgb_byte(4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s),
        srgb_byte(-1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s),
        srgb_byte(-0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s),
    )
}

pub fn hsl(hue: f32, saturation: f32, lightness: f32) -> LinearRGB {
    let hue = hue.rem_euclid(360.0) / 360.0;
    let saturation = saturation.clamp(0.0, 1.0);
    let lightness = lightness.clamp(0.0, 1.0);

    if saturation == 0.0 {
        let value = (lightness * 255.0).round() as u8;
        return rgb(value, value, value);
    }

    let q = if lightness < 0.5 {
        lightness * (1.0 + saturation)
    } else {
        lightness + saturation - lightness * saturation
    };
    let p = 2.0 * lightness - q;
    rgb(
        hsl_channel(p, q, hue + 1.0 / 3.0),
        hsl_channel(p, q, hue),
        hsl_channel(p, q, hue - 1.0 / 3.0),
    )
}

fn hsl_channel(p: f32, q: f32, mut hue: f32) -> u8 {
    if hue < 0.0 {
        hue += 1.0;
    }
    if hue > 1.0 {
        hue -= 1.0;
    }
    let value = if hue < 1.0 / 6.0 {
        p + (q - p) * 6.0 * hue
    } else if hue < 0.5 {
        q
    } else if hue < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - hue) * 6.0
    } else {
        p
    };
    (value * 255.0).round() as u8
}

fn srgb_byte(value: f32) -> u8 {
    let value = if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}
