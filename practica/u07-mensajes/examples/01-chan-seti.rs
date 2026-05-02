extern crate rand;

use std::collections::HashSet;
use std::thread;
use std::time::Duration;
use rand::{thread_rng, Rng};
use std::thread::JoinHandle;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

/*
PROBLEMA: una persona le presta dinero a varios amigos suyos, y cada uno busca invertir ese dinero para generar ganancias.
Esta persona le da la misma cantidad de dinero a cada amigo, y luego de un tiempo, cada amigo le devuelve las ganancias que obtuvo.
*/

const WORKERS: i32 = 10;

fn main() {
    // señal inicial (dinero prestado)
    let mut signal = 100.0;

    // Creamos el canal para recibir los resultados de los trabajadores, el coordinador recibirá por este canal los resultados que cada trabajador obtuvo. 
    let (result_send, result_receive) = mpsc::channel();

    // Creamos los trabajadores, cada uno con su canal para recibir la señal, y el canal para enviar los resultados
    let workers: Vec<(Sender<f64>, JoinHandle<()>)> = (0..WORKERS)
        .map(|id| {
            // cada trabajador tiene su propio canal para recibir la señal (worker_send, worker_receive), y el canal result_send para enviar los resultados al coordinador
            let (worker_send, worker_receive) = mpsc::channel();
            let result_send_worker = result_send.clone();
            let t = thread::spawn(move || worker(id, worker_receive, result_send_worker));
            (worker_send, t)
        })
        .collect();

    loop {
        let mut signal_epoch = start_epoch(&mut signal, &workers);

        let mut results = HashSet::new();

        // Espero a que todos por separado hayan enviado el resultado (evita contar dos veces el resultado de un mismo trabajador, o contar el resultado de un trabajador que no participó en esta época).
        while(results.len() < (WORKERS as usize)) {
            // El coordinador recibe las ganancias de cada trabajador, y las suma a la señal para la próxima época. Además, guarda qué trabajadores ya enviaron su resultado para no contar dos veces el mismo resultado.
            let (who, result) = result_receive.recv().unwrap();
            println!("[COORDINADOR] recibí de {} señal {}", who, result);
            // Si no había mandado entonces se guarda y suma la señal. Si ya había mandado, entonces se ignora el resultado.
            if !results.contains(&who) {
                results.insert(who);
                signal_epoch += result;
            }
        }

        println!("[COORDINADOR] señal final {}", signal_epoch);
        signal = signal_epoch
    }

    let _:Vec<()> = workers.into_iter()
        .flat_map(|(_,h)| h.join())
        .collect();
}

fn start_epoch(signal: &mut f64, workers: &Vec<(Sender<f64>, JoinHandle<()>)>) -> f64 {
    let signal_worker = *signal / (WORKERS as f64);
    // Nos habíamos guardado el extremo TX para cada trabajador, ahora le enviamos la señal a cada trabajador para que empiece a trabajar.
    for (worker, _) in workers {
        worker.send(signal_worker).unwrap();
    }

    // Devolvemos un 0 para que el coordinador vaya sumando las ganancias de cada trabajador, y así obtener la señal para la próxima época.
    let mut signal_epoch = 0.0;
    signal_epoch
}

fn worker(id: i32, signal_source: Receiver<f64>, result: Sender<(i32, f64)>) {
    loop {
        // Cada trabajador recibe la señal y hace algo con ella (en este caso, simplemente la multiplica por un número aleatorio entre 0 y 1).
        let signal = signal_source.recv().unwrap();
        println!("[WORKER {}] señal {}", id, signal);
        thread::sleep(Duration::from_secs(2));
        let resultado = signal * thread_rng().gen_range(0., 1.);
        println!("[WORKER {}] resultado {}", id, resultado);

        // El trabajador envía el resultado al coordinador, junto con su id para que el coordinador sepa quién envió el resultado. Esto se hace por el canal result, que es un canal compartido entre todos los trabajadores y el coordinador. El coordinador recibirá por este canal los resultados que cada trabajador obtuvo, y podrá sumar esos resultados a la señal para la próxima época.
        result.send((id, resultado));
    }
}