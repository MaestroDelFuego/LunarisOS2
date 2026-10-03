#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct FramebufferInfo {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u8,
}

pub fn default_target() -> FramebufferInfo {
    FramebufferInfo {
        width: 1280,
        height: 720,
        pitch: 1280 * 4,
        bpp: 32,
    }
}

pub fn choose_mode(
    target_width: u32,
    target_height: u32,
    modes: impl Iterator<Item = (u32, u32)>,
) -> (u32, u32) {
    let mut best = (0u32, 0u32);
    let mut best_distance = u64::MAX;

    for (width, height) in modes {
        let dx = width as i64 - target_width as i64;
        let dy = height as i64 - target_height as i64;
        let distance = (dx.abs() + dy.abs()) as u64;

        if distance < best_distance {
            best = (width, height);
            best_distance = distance;
        }
    }

    best
}
