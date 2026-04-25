extern crate rand;
extern crate num_derive;
extern crate num_traits;

use std::sync::{Arc, Mutex, Condvar};
use std::thread;
use std::time::Duration;

use rand::{thread_rng};
use rand::seq::SliceRandom;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::thread::JoinHandle;

fn main() {

    const N:usize = 3;

    #[derive(Clone, Copy, Debug, FromPrimitive)]
    enum Ingredients {
        Tobacco = 0,
        Paper,
        Fire
    }

    // Ahora creamos un par de Mutex y Condvar para sincronizar al agente y a los fumadores. El Mutex se utilizará para proteger el estado de los ingredientes disponibles sobre la mesa, y el Condvar se utilizará para que el agente y los fumadores puedan esperar y notificarse mutuamente cuando el estado de los ingredientes cambie.
    let pair = Arc::new((Mutex::new([false, false, false]), Condvar::new()));

    let pair_agent = pair.clone();

    let agent = thread::spawn(move || loop {
        let (lock, cvar) = &*pair_agent;

        println!("[Agente] Esperando a que fumen");
        // El agente espera a que los fumadores terminen de fumar antes de poner nuevos ingredientes sobre la mesa. Para esto, el agente adquiere el Mutex y verifica si hay ingredientes sobre la mesa (es decir, si algún ingrediente está disponible). Si hay ingredientes disponibles, el agente espera en el Condvar hasta que los fumadores terminen de fumar y liberen los ingredientes (actualizando el estado a false).
        let mut state = cvar.wait_while(lock.lock().unwrap(), |ings| {
            let full_table = (*ings).iter().any(|i| *i);
            println!("[Agente] Esperando a que fumen {:?} - {}", ings, full_table);
            full_table
        }).unwrap();

        let mut ings = vec!(Ingredients::Tobacco, Ingredients::Paper, Ingredients::Fire);
        ings.shuffle(&mut thread_rng());
        let selected_ings = &ings[0..N-1];

        // Se ponen los dos ingredientes seleccionados sobre la mesa, actualizando los estados de ambos a True.
        for ing in selected_ings {
            println!("[Agente] Pongo {:?}", ing);
            state[*ing as usize] = true;
        }
        // Se notifica a los fumadores que los ingredientes han cambiado, lo que les permitirá verificar si pueden fumar.
        cvar.notify_all();
    });

    let smokers:Vec<JoinHandle<()>> =  (0..N)
        .map(|fumador_id|  {
            let pair_smoker = pair.clone();
            let me = Ingredients::from_usize(fumador_id).unwrap();

            thread::spawn(move || loop {
                let (lock, cvar) = &*pair_smoker;
                // Cada fumador espera a que los dos ingredientes que necesita estén disponibles. Para esto, el fumador verifica cuáles son los ingredientes que necesita (los que no tiene) y espera en el Condvar hasta que esos ingredientes estén disponibles.
                let mut _guard = cvar.wait_while(lock.lock().unwrap(), |ings| {
                    // En .all() se busca los 'j' 
                    let my_turn = (0..N).all(|j| j == fumador_id || ings[j]);
                    println!("[Fumador {:?}] Chequeando {:?} - {}", me, ings, my_turn);
                    !my_turn
                }).unwrap();

                println!("[Fumador {:?}] Fumando", me);
                thread::sleep(Duration::from_secs(2));
                for ing in (*_guard).iter_mut() {
                    *ing = false;
                }
                println!("[Fumador {:?}] Terminé", me);
                cvar.notify_all();
            })
        })
        .collect();

    let _:Vec<()> = smokers.into_iter()
        .flat_map(|x| x.join())
        .collect();

    agent.join().unwrap();
}