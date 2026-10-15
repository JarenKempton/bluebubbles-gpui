use gpui_kit::{Hsla, Rgba};

#[derive(Clone, Copy)]
pub struct Oklch {
    pub l: f32,
    pub c: f32,
    pub h: f32,
}

fn to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(c: f32) -> f32 {
    let c = c.clamp(0., 1.);
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1. / 2.4) - 0.055
    }
}

pub fn from_hsla(color: Hsla) -> Oklch {
    let rgb = color.to_rgb();
    let (r, g, b) = (to_linear(rgb.r), to_linear(rgb.g), to_linear(rgb.b));
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    let lab_l = 0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s;
    let a = 1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s;
    let lab_b = 0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s;
    Oklch {
        l: lab_l,
        c: a.hypot(lab_b),
        h: lab_b.atan2(a).to_degrees().rem_euclid(360.),
    }
}

pub fn to_hsla(color: Oklch) -> Hsla {
    let (a, b) = (
        color.c * color.h.to_radians().cos(),
        color.c * color.h.to_radians().sin(),
    );
    let l = (color.l + 0.396_337_78 * a + 0.215_803_76 * b).powi(3);
    let m = (color.l - 0.105_561_346 * a - 0.063_854_17 * b).powi(3);
    let s = (color.l - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);
    Rgba {
        r: from_linear(4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s),
        g: from_linear(-1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s),
        b: from_linear(-0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s),
        a: 1.,
    }
    .into()
}

pub struct Ramp {
    canvas: Oklch,
    accent: Oklch,
    hue: f32,
    tint: f32,
    step: f32,
}

impl Ramp {
    pub fn new(canvas: Hsla, accent: Hsla, dark: bool) -> Self {
        let (canvas, accent) = (from_hsla(canvas), from_hsla(accent));
        Self {
            hue: if accent.c < 0.02 { canvas.h } else { accent.h },
            tint: (accent.c * 0.22).clamp(0.008, 0.045),
            step: if dark { 1. } else { -1. },
            canvas,
            accent,
        }
    }

    pub fn surface(&self, delta_l: f32, accent_share: f32, cap: f32) -> Hsla {
        let chroma = if accent_share > 0. {
            (self.accent.c * accent_share).min(cap)
        } else {
            self.tint
        };
        to_hsla(Oklch {
            l: (self.canvas.l + self.step * delta_l).clamp(0.05, 0.98),
            c: chroma,
            h: self.hue,
        })
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::rgb;

    use super::{from_hsla, to_hsla};

    #[test]
    fn round_trips_srgb() {
        let color = rgb(0x7aa2f7).into();
        let back = to_hsla(from_hsla(color)).to_rgb();
        let original = color.to_rgb();
        assert!((back.r - original.r).abs() < 0.002);
        assert!((back.b - original.b).abs() < 0.002);
    }
}
