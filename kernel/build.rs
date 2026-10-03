use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let image_path = manifest_dir.parent().unwrap().join("canvas.png");
    println!("cargo:rerun-if-changed={}", image_path.display());

    let file = File::open(&image_path)?;
    let mut decoder = png::Decoder::new(file);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut input = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut input)?;
    let input = &input[..info.buffer_size()];

    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return Err("PNG palette was not expanded".into()),
    };

    let mut blt_pixels = Vec::with_capacity(info.width as usize * info.height as usize * 4);
    for pixel in input.chunks_exact(channels) {
        let (red, green, blue) = match info.color_type {
            png::ColorType::Grayscale | png::ColorType::GrayscaleAlpha => {
                (pixel[0], pixel[0], pixel[0])
            }
            png::ColorType::Rgb | png::ColorType::Rgba => (pixel[0], pixel[1], pixel[2]),
            png::ColorType::Indexed => unreachable!(),
        };
        blt_pixels.extend_from_slice(&[blue, green, red, 0]);
    }

    let output_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(output_dir.join("canvas.blt"), blt_pixels)?;
    fs::write(
        output_dir.join("canvas_meta.rs"),
        format!(
            "pub const WIDTH: usize = {};\npub const HEIGHT: usize = {};\n",
            info.width, info.height
        ),
    )?;

    Ok(())
}
