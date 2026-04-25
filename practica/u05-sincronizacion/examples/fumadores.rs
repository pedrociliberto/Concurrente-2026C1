extern crate rand;
extern crate std_semaphore;
extern crate num_derive;
extern crate num_traits;

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rand::{thread_rng};
use rand::seq::SliceRandom;
use std_semaphore::Semaphore;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::thread::JoinHandle;

/*
PROBLEMA: tenemos 3 fumadores, cada uno tiene cantidades infinitas de uno de los ingredientes necesarios para fumar (tabaco, papel o fuego). Un agente pone dos de los ingredientes sobre la mesa, y el fumador que los necesita los tomará para fumar. El agente no puede poner nuevos ingredientes hasta que el fumador que los tomó termine de fumar.
*/

const N:usize = 3;

// Definimos un enum para los 3 ingredientes.
#[derive(Clone, Copy, Debug, FromPrimitive)]
enum Ingredients {
    Tobacco = 0,
    Paper,
    Fire
}

const ALL_INGREDIENTS: [Ingredients; N] = [Ingredients::Tobacco, Ingredients::Paper, Ingredients::Fire];

fn main() {
    // Creamos un semáforo para el agente, que se utilizará para controlar cuándo el agente puede poner nuevos ingredientes sobre la mesa. El agente comenzará con el semáforo disponible (valor 1), lo que le permitirá poner los primeros ingredientes.
    let agent_sem = Arc::new(Semaphore::new(1));
    // Creamos un vector de semáforos para los ingredientes, que se utilizarán para que los fumadores esperen a que los ingredientes que necesitan estén disponibles. Cada semáforo comenzará con el valor 0, lo que significa que inicialmente no hay ingredientes disponibles.
    let ingredient_sems: Arc<Vec<Semaphore>> = Arc::new((0..N)
                                       .map(|_| Semaphore::new(0))
                                       .collect());
    // Clonamos los semáforos para pasar a los hilos.
    let agent_sem_a = agent_sem.clone();
    let ingredients_sem_a = ingredient_sems.clone();

    let agent = thread::spawn(move || loop {
        println!("[Agente] Esperando sem");
        // Se adquiere el semáforo del agente, lo que bloquea al agente hasta que el fumador que tomó los ingredientes termine de fumar y libere el semáforo.
        agent_sem_a.acquire();

        // Se seleccionan aleatoriamente dos ingredientes para poner sobre la mesa. Esto se hace mezclando el vector de ingredientes y tomando los primeros dos.
        let mut ings = ALL_INGREDIENTS.to_vec();
        ings.shuffle(&mut thread_rng());
        let selected_ings = &ings[0..N-1];
        for ing in selected_ings {
            println!("[Agente] Pongo {:?}", ing);
            // Se liberan los semáforos correspondientes a los ingredientes seleccionados, lo que permite que el fumador que necesita esos ingredientes pueda adquirirlos y comenzar a fumar.
            ingredients_sem_a[*ing as usize].release();
        }
    });

    let smokers:Vec<JoinHandle<()>> = (0..N)
        .map(|i|  {
            let agent_sem_smoker = agent_sem.clone();
            let ingredient_sems_smoker = ingredient_sems.clone();
            // Creo un hilo para cada fumador.
            thread::spawn(move || loop {
                let me = Ingredients::from_usize(i).unwrap();
                // Cada fumador espera a que los dos ingredientes que necesita estén disponibles. Para esto, el fumador verifica cuáles son los ingredientes que necesita (los que no tiene) y adquiere los semáforos correspondientes a esos ingredientes.
                for ing_id in 0..N {
                    if ing_id != i {
                        let ing = Ingredients::from_usize(ing_id).unwrap();
                        println!("[Fumador {:?}] Esperando {:?}", me, ing);
                        // El fumador adquiere el semáforo del ingrediente que necesita, lo que bloquea al fumador hasta que el agente ponga ese ingrediente sobre la mesa.
                        ingredient_sems_smoker[ing_id].acquire();
                        println!("[Fumador {:?}] Obtuve {:?}", me, ing);
                    }
                }
                println!("[Fumador {:?}] Fumando", me);
                thread::sleep(Duration::from_secs(2));
                // Una vez que el fumador termina de fumar, libera el semáforo del agente para permitir que el agente ponga nuevos ingredientes sobre la mesa.
                agent_sem_smoker.release();
                println!("[Fumador {:?}] Terminé", me);
            })
        })
        .collect();

    let _:Vec<()> = smokers.into_iter()
        .flat_map(|x| x.join())
        .collect();

    agent.join().unwrap();
}

// Aquí es posible CAER EN DEADLOCK, ya que dos fumadores podrían adquirir un semáforo de ingrediente cada uno y luego quedar bloqueados esperando el otro ingrediente que el agente no puede poner porque el semáforo del agente está bloqueado por los fumadores.