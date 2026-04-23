extern crate std_semaphore;
extern crate rand;

use std_semaphore::Semaphore;
use std::thread;
use std::time::Duration;
use rand::{thread_rng, Rng};
use std::sync::Arc;
use std::thread::JoinHandle;

const N:usize = 5;

/**
Cinco filósofos se sientan alrededor de una mesa y pasan su vida cenando y pensando.
Cada filósofo tiene un plato de fideos y un palito chino a la izquierda de su plato.
Para comer los fideos son necesarios dos palitos y cada filósofo sólo puede tomar los que
están a su izquierda y derecha. Si cualquier filósofo toma un palito y el otro está ocupado,
se quedará esperando, con el tenedor en la mano, hasta que pueda tomar el otro tenedor,
para luego empezar a comer.
*/

fn main() {
    // Creamos un VECTOR DE SEMÁFOROS, cada semáforo representa un palito chino, y el valor del semáforo representa si el palito está disponible (1) o no (0). El vector es ARC, lo que permite compartirlo entre los hilos de los filósofos de manera segura.
    // El vector tiene longitud N (cantidad de comensales), y cada semáforo se inicializa con el valor 1 (disponible).
    let chopsticks:Arc<Vec<Semaphore>> = Arc::new((0 .. N)
        .map(|_| Semaphore::new(1))
        .collect());

    // Creamos un VECTOR DE HILOS, cada hilo representa a un filósofo. Cada filósofo tiene un ID (de 0 a N-1) y una referencia al vector compartido de semáforos (palitos).
    let philosophers:Vec<JoinHandle<()>> = (0 .. N)
        .map(|id| {
            let chopsticks_local = chopsticks.clone();
            thread::spawn(move || philosopher(id, chopsticks_local))
        })
        .collect();

    // Esperamos a que todos los hilos de los filósofos terminen (en este caso, el programa corre indefinidamente, así que esto no sucederá, pero es una buena práctica).
    for philosopher in philosophers {
        philosopher.join();
    }

}

/// Función que representa el comportamiento de cada filósofo. Cada filósofo intenta tomar los palitos a su izquierda y derecha para comer, y luego vuelve a pensar. Mientras el filósofo no tiene ambos palitos, se queda esperando (pensando).
fn philosopher(id: usize, chopsticks: Arc<Vec<Semaphore>>) {
    // Calculamos el índice del palito a la derecha del filósofo (el siguiente en la mesa, con wrap-around usando módulo).
    let next = (id + 1) % N;
    let first_chopstick;
    let second_chopstick;

    // Solucion al deadlock: para evitar que todos los filósofos tomen el palito izquierdo al mismo tiempo y queden bloqueados esperando el derecho, hacemos que el último filósofo (con ID N-1) tome primero el palito derecho y luego el izquierdo, mientras que los demás filósofos toman primero el palito izquierdo y luego el derecho. Esto rompe la simetría y evita el deadlock.
    // Si el último toma el palito derecho primero en vez del izquierdo, entonces el filósofo N-2 podrá tomar el palito izquierdo y también el derecho (que es el mismo que el izquierdo del filósofo N-1), ya que el último no lo agarró, y así podrá comer por primera vez, liberando la posibilidad de que se enfrenten a un deadlock.
    if id == (N-1) {
       first_chopstick = &chopsticks[next];
       second_chopstick = &chopsticks[id];
    } else {
       first_chopstick = &chopsticks[id];
       second_chopstick = &chopsticks[next];
    }

    // Tratamos de forzar que los filósofos intenten tomar el primer palito en el orden de su ID, para aumentar la probabilidad de que se produzca un deadlock (si no se soluciona). Esto se hace haciendo que cada filósofo espere un tiempo proporcional a su ID antes de intentar tomar el primer palito.
    // El filósofo con ID 0 intentará tomar el primer palito inmediatamente, el filósofo con ID 1 esperará un poco más, y así sucesivamente, lo que aumenta la probabilidad de que todos los filósofos tomen su primer palito al mismo tiempo y queden bloqueados esperando el segundo palito.
    thread::sleep(Duration::from_millis(100 * id as u64));

    loop {
        println!("filosofo {} pensando", id);
        //thread::sleep(Duration::from_millis(thread_rng().gen_range(500, 1500)));
        println!("filosofo {} esperando palito izquierdo", id);
        {
            // Intentamos tomar el primer palito (el que se asignó como "first_chopstick"). Si el semáforo del palito está en 1 (disponible), lo tomamos (lo ponemos en 0) y continuamos. Si el semáforo está en 0 (ocupado), el filósofo se queda esperando hasta que el semáforo vuelva a estar en 1.
            let first_access = first_chopstick.access();
            // Pausa despues del primer palito para forzar posible Deadlock (si no lo handleamos)
            thread::sleep(Duration::from_millis(1000));
            println!("filosofo {} esperando palito derecho", id);
            {
                // Intentamos tomar el segundo palito (el que se asignó como "second_chopstick"). Si el semáforo del palito está en 1 (disponible), lo tomamos (lo ponemos en 0) y continuamos. Si el semáforo está en 0 (ocupado), el filósofo se queda esperando hasta que el semáforo vuelva a estar en 1.
                let second_access = second_chopstick.access();
                println!("filosofo {} comiendo", id);
                thread::sleep(Duration::from_millis(thread_rng().gen_range(500, 1500)));
            }
            // Al salir del bloque de código donde se accedió al palito derecho, el semáforo del segundo palito se libera (se pone en 1), lo que permite que otros filósofos que estén esperando ese palito puedan tomarlo. 
        }
        // Lo mismo con el palito izquierdo: el semáforo se libera (se pone en 1), para que otros filósofos puedan tomarlo.
    }
}