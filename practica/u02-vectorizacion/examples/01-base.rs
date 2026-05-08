extern crate image;

use std::ptr;
use std::time::Instant;

use image::{GenericImageView, ImageBuffer, Rgb, RgbImage};

/*
Ejemplo introductorio pre-Vectorización.
En este ejemplo, se carga una imagen, se convierte a escala de grises y se guarda el resultado.
Aquí el proceso es secuencial, sin optimizaciones.
*/

fn main() {
    // Cargar la imagen de entrada
    let input_path = concat!(env!("CARGO_MANIFEST_DIR"), "/data/totk.jpg");
    let input_image = image::open(input_path).unwrap().to_rgb8();

    let (width, height) = input_image.dimensions();
    let mut output_image:RgbImage = ImageBuffer::new(width, height);

    let start = Instant::now();

    // Convertir a escala de grises, proceso secuencial
    for y in 0..height {
        for x in 0..width {
            let pixel = input_image.get_pixel(x, y);
            let grayscale_value = (pixel[0] as f32 * 0.299 + pixel[1] as f32 * 0.587 + pixel[2] as f32 * 0.114) as u8;
            output_image.put_pixel(x, y, Rgb([grayscale_value, grayscale_value, grayscale_value]));
        }
    }

    println!("{:?}", start.elapsed());

    output_image.save(concat!(env!("CARGO_MANIFEST_DIR"), "/target/output.jpg")).unwrap();

}