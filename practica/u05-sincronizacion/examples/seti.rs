extern crate rand;

use std::sync::{Arc, RwLock, Barrier};
use std::thread;
use std::time::Duration;
use rand::Rng;

/*
PROBLEMA: El proyecto SETI@home es un proyecto de computación distribuida que utiliza el tiempo de CPU de los voluntarios para analizar datos de radioastronomía en busca de señales de inteligencia extraterrestre. En este proyecto, cada voluntario ejecuta un programa que procesa una parte de los datos y devuelve los resultados al servidor central.

- Todos los procesadores inician con el mismo valor de señal, 1/N del total, donde N es el número de procesadores disponibles.
- Al final del procesamiento, se suman los resultados de cada procesador y se actualiza el valor de la señal para la siguiente ronda de procesamiento.
- El procesamiento individual consiste de generar un número aleatorio entre 0 y 1, multiplicarlo por la señal actual y devolver el resultado al servidor central.
*/

const WORKERS: u32 = 10;

fn main() {
    // Definimos un valor inicial de señal.
    let signal: f64 = 100.0;
    // Creamos un RwLock para proteger el acceso a la señal compartida entre los trabajadores.
    let lock = Arc::new(RwLock::new(signal));
    // Creamos dos barreras para sincronizar a los trabajadores.
    let barrier = Arc::new(Barrier::new(WORKERS as usize));
    let barrier2 = Arc::new(Barrier::new(WORKERS as usize));

    let mut workers = vec![];

    for id in 0..WORKERS {
        let lock_clone = lock.clone();
        let barrier_clone = barrier.clone();
        let barrier2_clone = barrier2.clone();
        // Le pasamos a cada trabajador una copia de los Arc para que puedan acceder a la señal y sincronizarse entre ellos.
        workers.push(thread::spawn(move || worker(id, lock_clone, barrier_clone, barrier2_clone)));
    }

    for worker in workers {
        worker.join().unwrap();
    }

}

fn worker(id: u32, lock: Arc<RwLock<f64>>, barrier: Arc<Barrier>, barrier2: Arc<Barrier>) {
    let mut epoch = 0;

    loop {
        // Primer barrera: espero a que todos los trabajadores inicien la vuelta.
        // hacemos esto para asegurarnos de que todos los trabajadores lean el mismo valor de la señal antes de que alguno de ellos la modifique.
        barrier.wait();

        let signal = *lock.read().unwrap() / WORKERS as f64;
        println!("[WORKER {}] inicio epoch {} signal {}", id, epoch, signal);
        
        // Segunda barrera: espero a que todos los trabajadores hayan leído el saldo disponible antes de que alguno de ellos lo modifique.
        barrier2.wait();

        // Tomo el dinero: cada trabajador resta su parte de la señal del total.
        if let Ok(mut money_guard) = lock.write() {
            *money_guard -= signal;
        }

        epoch += 1;

        // Se simula el procesamiento.
        let mut rng = rand::thread_rng();
        let random_result: f64 = rng.gen_range(0.0, 1.0);
        thread::sleep(Duration::from_millis((2000 as f64 * random_result) as u64));
        let result = signal * random_result;
        println!("[WORKER {}] voy a retornar {}", id, result);

        // Cada trabajador devuelve su resultado al servidor central, sumándolo a la señal para la siguiente ronda de procesamiento.
        if let Ok(mut guard) = lock.write() {
            *guard += result;
        }

        println!("[WORKER] {} retorné {}", id, result);
    }
}
