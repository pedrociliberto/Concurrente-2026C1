extern crate num_derive;
extern crate num_traits;
extern crate rand;

use std::cell::UnsafeCell;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use rand::{Rng, thread_rng};
use rand::seq::SliceRandom;

/*
PROBLEMA: Un estado se comparte entre varios procesos.
Algunos procesos necesitan actualizar el estado (escritores) y otros solo necesitan leerlo (lectores). 
Se quiere permitir que varios lectores lean el estado al mismo tiempo, pero si un escritor está actualizando el estado, ningún otro proceso (ni lector ni escritor) puede acceder a él.
*/

// Definimos un Struct para mantener el estado de los lectores y escritores. En este caso, tenemos un contador de lectores activos y un booleano que indica si hay un escritor activo.
#[derive(Debug)]
struct ReadWrite {
    readers: i32,
    writing: bool
}

// Usamos Unsafe para definir el valor que compartirán los lectores y escritores. 
// Igual, en la materia no usamos bloques Unsafe.
struct DataHolder {
    data: UnsafeCell<i32>
}
unsafe impl Sync for DataHolder {}


fn main() {
    const READERS: i32 = 5;
    const WRITERS: i32 = 2;

    // Creamos un par de Mutex y Condvar para sincronizar a los lectores y escritores. El Mutex se utilizará para proteger el estado de los lectores y escritores, y el Condvar se utilizará para que los lectores y escritores puedan esperar y notificarse mutuamente cuando el estado cambie.
    let pair = Arc::new((Mutex::new(ReadWrite { readers: 0, writing: false }), Condvar::new()));
    // Creamos nuestro valor compartido: un número entero (42) que los escritores actualizarán y los lectores leerán.
    let data = Arc::new(DataHolder { data: UnsafeCell::new(42) } );

    let readers: Vec<JoinHandle<()>> = (0..READERS)
        .map(|me| {
            let pair_reader = pair.clone();
            let data_reader = data.clone();

            thread::spawn(move || loop {
                let (lock, cvar) = &*pair_reader;

                // Sacar esto para llegar a starvation del writer
                // thread::sleep(Duration::from_millis(thread_rng().gen_range(500, 1500)));
                {
                    // Esperamos a que nadie esté escribiendo, para poder leer el valor.
                    let mut _guard = cvar.wait_while(lock.lock().unwrap(), |state| {
                        println!("[Lector {}] Chequeando {:?}", me, state);
                        state.writing
                    }).unwrap();
                    _guard.readers += 1;
                }

                unsafe {
                    println!("[Lector {:?}] Leyendo {}", me, data_reader.data.get().read());
                }
                thread::sleep(Duration::from_millis(thread_rng().gen_range(500, 1500)));
                println!("[Lector {:?}] Terminé", me);

                // Cuando termina de leer, decrementa el contador de lectores activos y notifica a los escritores que podrían estar esperando a que no haya lectores activos para escribir.
                lock.lock().unwrap().readers -= 1;
                cvar.notify_all();
            })
        })
        .collect();

    let writers: Vec<JoinHandle<()>> = (0..WRITERS)
        .map(|me| {
            let pair_writer = pair.clone();
            let data_writer = data.clone();

            thread::spawn(move || loop {
                let (lock, cvar) = &*pair_writer;

                {
                    // Esperamos a que no haya lectores ni escritores activos, para poder escribir el valor.
                    let mut _guard = cvar.wait_while(lock.lock().unwrap(), |state| {
                        println!("[Escritor {}] Chequeando {:?}", me, state);
                        state.writing || state.readers > 0
                    }).unwrap();
                    _guard.writing = true;
                }

                unsafe {
                    println!("[Escritor {:?}] Escribiendo", me);
                    data_writer.data.get().write(me);
                }
                thread::sleep(Duration::from_millis(thread_rng().gen_range(500, 1500)));
                println!("[Escritor {:?}] Terminé", me);

                // Una vez termina de escribir, actualiza el estado para indicar que ya no hay un escritor activo y notifica a los lectores y escritores que podrían estar esperando a que el estado cambie.
                lock.lock().unwrap().writing = false;
                cvar.notify_all();
            })
        })
        .collect();

    let _:Vec<()> = readers.into_iter()
        .chain(writers.into_iter())
        .flat_map(|x| x.join())
        .collect();

}

/* 
Acá se da que SIEMPRE HAY LECTORES, y nunca da posibilidad a que un escritor escriba, ya que cada vez que un escritor quiere modificar el valor, se bloquea esperando a que no haya lectores.
Para evitar esto, podemos descomentar la línea del SLEEP, para que las lecturas sean más esporádicas.
Si no, ver el archivo 'lector-escritor-fair.rs' para una solución que garantiza que tanto lectores como escritores puedan acceder al recurso de manera justa.
*/