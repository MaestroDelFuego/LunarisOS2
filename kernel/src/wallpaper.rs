use uefi::proto::console::gop::{BltOp, BltPixel, BltRegion, GraphicsOutput};

mod metadata {
    include!(concat!(env!("OUT_DIR"), "/canvas_meta.rs"));
}

const IMAGE_LEN: usize = include_bytes!(concat!(env!("OUT_DIR"), "/canvas.blt")).len();

#[repr(align(4))]
struct AlignedPixels<const N: usize>([u8; N]);

static IMAGE: AlignedPixels<IMAGE_LEN> =
    AlignedPixels(*include_bytes!(concat!(env!("OUT_DIR"), "/canvas.blt")));

pub fn draw(
    gop: &mut GraphicsOutput,
    screen_width: usize,
    screen_height: usize,
    clip: Option<myos_gui_core::Rect>,
) -> uefi::Result {
    let source_pixels = unsafe {
        core::slice::from_raw_parts(
            IMAGE.0.as_ptr().cast::<BltPixel>(),
            metadata::WIDTH * metadata::HEIGHT,
        )
    };

    let screen_is_wider = screen_width * metadata::HEIGHT >= screen_height * metadata::WIDTH;
    let (crop_x, crop_y, crop_width, crop_height) = if screen_is_wider {
        let crop_height = (metadata::WIDTH * screen_height / screen_width).max(1);
        (
            0,
            (metadata::HEIGHT - crop_height) / 2,
            metadata::WIDTH,
            crop_height,
        )
    } else {
        let crop_width = (metadata::HEIGHT * screen_width / screen_height).max(1);
        (
            (metadata::WIDTH - crop_width) / 2,
            0,
            crop_width,
            metadata::HEIGHT,
        )
    };

    const ROW_CAPACITY: usize = 4096;
    let mut row = [BltPixel::new(0, 0, 0); ROW_CAPACITY];

    let (start_x, end_x, start_y, end_y) = if let Some(clip) = clip {
        (
            clip.x.max(0) as usize,
            (i64::from(clip.x) + i64::from(clip.width)).clamp(0, screen_width as i64) as usize,
            clip.y.max(0) as usize,
            (i64::from(clip.y) + i64::from(clip.height)).clamp(0, screen_height as i64) as usize,
        )
    } else {
        (0, screen_width, 0, screen_height)
    };

    for y in start_y.min(screen_height)..end_y {
        let source_y = crop_y + y * crop_height / screen_height;
        let mut x = start_x.min(screen_width);

        while x < end_x {
            let count = (end_x - x).min(ROW_CAPACITY);
            for offset in 0..count {
                let screen_x = x + offset;
                let source_x = crop_x + screen_x * crop_width / screen_width;
                row[offset] = source_pixels[source_y * metadata::WIDTH + source_x];
            }

            gop.blt(BltOp::BufferToVideo {
                buffer: &row[..count],
                src: BltRegion::Full,
                dest: (x, y),
                dims: (count, 1),
            })?;
            x += count;
        }
    }

    Ok(())
}
