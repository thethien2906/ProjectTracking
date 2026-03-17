use std::env;
use std::path::Path;
use image::ImageFormat;

fn main() {
    // Only compile resources on Windows
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
        let png_path = Path::new("assets").join("logo.png");
        let ico_path = Path::new(&out_dir).join("icon.ico");

        // Convert PNG to ICO for the executable resource
        if png_path.exists() {
            let img = image::open(&png_path).expect("Failed to open logo.png");
            // Resize to 256x256 which is the maximum allowed for Windows ICO files
            let resized = img.resize_exact(256, 256, image::imageops::FilterType::Lanczos3);
            resized.save_with_format(&ico_path, ImageFormat::Ico)
                .expect("Failed to convert PNG to ICO");

            // Embed the ICO as the main EXE icon
            let mut res = winresource::WindowsResource::new();
            res.set_icon(ico_path.to_str().expect("Valid UTF-8 path"));
            res.compile().expect("Failed to compile Windows resource");
        }
    }
}
