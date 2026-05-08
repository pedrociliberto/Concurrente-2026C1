extern crate image;

use std::ptr;
use std::time::Instant;

use image::{GenericImageView, ImageBuffer, RgbImage};

#[derive(Copy, Clone)]
struct SendPointer(*mut u8);
unsafe impl Sync for SendPointer {

}

unsafe impl Send for SendPointer {

}

/*
Aquí se muestra un ejemplo de cómo usar Rayon para paralelizar el proceso de conversión a escala de grises.
En este ejemplo, se carga la misma imagen que en el ejemplo secuencial, pero el proceso de conversión se realiza en paralelo usando Rayon.
Cada hilo procesa una fila de la imagen, y se escribe directamente en el buffer de salida usando punteros.
*/

fn main() {

    let input_path = concat!(env!("CARGO_MANIFEST_DIR"), "/data/totk.jpg");
    let input_image = &image::open(input_path).unwrap().to_rgb8();

    let (width, height) = input_image.dimensions();
    let mut output_image:RgbImage = ImageBuffer::new(width, height);
    let nasty_output = SendPointer(output_image.as_mut_ptr());

    // Prepara el pool de hilos de Rayon para usarlo en el scope. Detecta cuántos núcleos tiene el sistema y crea esa cantidad de hilos.
    rayon::ThreadPoolBuilder::new().build_global();

    let start = Instant::now();

    // Usamos rayon::scope para crear un ámbito de ejecución paralelo. Dentro de este ámbito, podemos lanzar tareas que se ejecutarán en paralelo.
    // 's' es el ámbito de ejecución paralelo, y 't' es el ámbito de cada tarea individual. En este caso, cada tarea procesa una fila de la imagen.
    rayon::scope(|s| {
        // Para cada fila de la imagen, lanzamos una tarea en paralelo usando 's.spawn'. Cada tarea procesa una fila completa de la imagen.
        for y in 0..height {
            s.spawn(move |t| {
                for x in 0..width {
                    let pixel = input_image.get_pixel(x, y);
                    // Aquí, en lugar de escribir directamente en el buffer de salida usando output_image.put_pixel, usamos punteros para escribir directamente en la memoria del buffer de salida.
                    // Si usamos t.spawn, cada tarea se ejecutará en paralelo, y cada tarea escribirá en su propia fila de la imagen de salida. Esto evita conflictos de escritura entre tareas.
                    //t.spawn(move |_| {
                        let grayscale_value = (pixel[0] as f32 * 0.299 + pixel[1] as f32 * 0.587 + pixel[2] as f32 * 0.114) as u8;
                        let coord = ((y * width + x) * 3) as isize;
                        unsafe {
                            ptr::write_bytes(nasty_output.0.offset(coord), grayscale_value, 3);
                        }
                    //})
                }
            })
        }
    });

    println!("{:?}", start.elapsed());

    output_image.save(concat!(env!("CARGO_MANIFEST_DIR"), "/target/output.jpg")).unwrap();

}