extern crate rand;
extern crate std_semaphore;

use std::sync::{Arc, Barrier, Mutex, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use std_semaphore::Semaphore;
use rand::{thread_rng, Rng};
use std::sync::atomic::{AtomicI32, Ordering};

fn main() {
    // Definimos la cantidad de clientes que pueden esperar en la barbería (N).
    const N: usize = 5;

    // Semáforo para indicar que hay un cliente esperando para ser atendido por el barbero. Cuando esté en 1, significa que hay un cliente esperando; cuando esté en 0, no hay clientes esperando.
    let customer_waiting = Arc::new(Semaphore::new(0));
    // Semáforo para indicar que el barbero está listo para atender a un cliente. Cuando esté en 1, significa que el barbero está listo; cuando esté en 0, el barbero no está listo.
    let barber_ready = Arc::new(Semaphore::new(0));
    // Semáforo para indicar que el corte de pelo ha terminado. Cuando esté en 1, significa que el corte de pelo ha terminado; cuando esté en 0, el corte de pelo no ha terminado.
    let haircut_done = Arc::new(Semaphore::new(0));
    // Mutex para proteger el acceso a la variable que indica el cliente actual que está siendo atendido por el barbero. El valor de esta variable es el ID del cliente actual, o -1 si no hay ningún cliente siendo atendido.
    let current_customer = Arc::new(Mutex::new(-1));
    // Semáforo para indicar que el cliente actual ha sido asignado al barbero. Cuando esté en 1, significa que el cliente actual ha sido asignado; cuando esté en 0, el cliente actual no ha sido asignado.
    let current_customer_set = Arc::new(Semaphore::new(0));

    // Se clonan las referencias a los semáforos y mutex para pasarlos al hilo del barbero.
    let customer_waiting_barber = customer_waiting.clone();
    let barber_ready_barber = barber_ready.clone();
    let haircut_done_barber = haircut_done.clone();
    let current_customer_barber = current_customer.clone();
    let current_customer_set_barber = current_customer_set.clone();

    // Se crea el hilo del barbero, que ejecuta un bucle infinito donde espera a que haya un cliente disponible, luego se prepara para atenderlo, asigna al cliente actual, corta el pelo y finalmente indica que ha terminado.
    let barber = thread::spawn(move || loop {
        // El barbero espera a que un cliente indique que está esperando (customer_waiting.acquire()). Esto bloquea al barbero hasta que un cliente libere el semáforo.
        println!("[Barbero] Esperando cliente");
        customer_waiting_barber.acquire();

        // Una vez que un cliente ha indicado que está esperando, el barbero se prepara para atenderlo liberando el semáforo barber_ready (barber_ready_barber.release()), lo que indica al cliente que el barbero está listo.
        println!("[Barbero] Cliente encontrado, preparándome para atenderlo");
        barber_ready_barber.release();

        // El barbero espera a que el cliente actual sea asignado (current_customer_set_barber.acquire()). Esto bloquea al barbero hasta que un cliente libere el semáforo después de asignarse a sí mismo como cliente actual.
        current_customer_set_barber.acquire();
        println!("[Barbero] Cortando pelo a {}", current_customer_barber.lock().unwrap());

        // Simulamos el corte de pelo con una pausa de 2 segundos.
        thread::sleep(Duration::from_secs(2));

        // Una vez que el corte de pelo ha terminado, el barbero libera el semáforo haircut_done (haircut_done_barber.release()), lo que indica al cliente que su corte de pelo ha terminado.
        haircut_done_barber.release();
        println!("[Barbero] Terminé");
    });

    // Se crea un mutex para asignar IDs únicos a los clientes que llegan a la barbería. El valor inicial es 1, y cada cliente incrementará este valor para obtener su ID único.
    let customer_id = Arc::new(Mutex::new(1));
    
    // Se crean los hilos de los clientes. Cada cliente ejecuta un bucle infinito donde espera un tiempo aleatorio antes de entrar a la barbería, luego indica que ha entrado, espera a que el barbero esté listo, se asigna a sí mismo como cliente actual, espera a que el corte de pelo termine y finalmente indica que su corte de pelo ha terminado.
    let customers: Vec<JoinHandle<()>> = (0..(N+1))
        .map(|_| {
            let barber_ready_customer = barber_ready.clone();
            let customer_waiting_customer = customer_waiting.clone();
            let haircut_done_customer = haircut_done.clone();
            let current_customer_id_customer = current_customer.clone();
            let current_customer_set_customer = current_customer_set.clone();
            let customer_id_customer = customer_id.clone();

            thread::spawn(move || loop {
                thread::sleep(Duration::from_secs(thread_rng().gen_range(2, 10)));

                // Cada cliente obtiene un ID único al bloquear el mutex customer_id, incrementar su valor y luego desbloquearlo. Esto garantiza que cada cliente tenga un ID distinto.
                let me = { 
                    let mut current = customer_id_customer.lock().unwrap();
                    *current += 1;
                    *current
                };

                // El cliente indica que está esperando para ser atendido por el barbero liberando el semáforo customer_waiting (customer_waiting_customer.release()). Esto puede despertar al barbero si está esperando a que un cliente llegue.
                println!("[Cliente {}] Entro a la barberia", me);
                customer_waiting_customer.release();

                // El cliente espera a que el barbero esté listo para atenderlo (barber_ready_customer.acquire()). Esto bloquea al cliente hasta que el barbero libere el semáforo indicando que está listo.
                println!("[Cliente {}] Esperando barbero", me);
                barber_ready_customer.acquire();

                // El cliente se asigna a sí mismo como el cliente actual bloqueando el mutex current_customer, estableciendo su ID como el cliente actual, y luego desbloqueando el mutex. Esto permite que el barbero sepa qué cliente está siendo atendido.
                println!("[Cliente {}] Me siento en la silla del barbero", me);
                *current_customer_id_customer.lock().unwrap() = me;

                // El cliente indica que se ha asignado a sí mismo como cliente actual liberando el semáforo current_customer_set (current_customer_set_customer.release()). Esto hará que el barbero pueda comenzar el corte de pelo.
                current_customer_set_customer.release();

                // El cliente espera a que el corte de pelo termine (haircut_done_customer.acquire()). Esto bloquea al cliente hasta que el barbero libere el semáforo indicando que el corte de pelo ha terminado.
                println!("[Cliente {}] Esperando a que me termine de cortar", me);
                haircut_done_customer.acquire();

                println!("[Cliente {}] Me terminaron de cortar", me);
            })
        })
        .collect();

    // Esperamos a que todos los hilos de los clientes terminen. En este caso, los hilos de los clientes ejecutan un bucle infinito, por lo que esto no sucederá, pero es una buena práctica esperar a que los hilos terminen antes de finalizar el programa.
    let _:Vec<()> = customers.into_iter()
        .flat_map(|x| x.join())
        .collect();

    // Esperamos a que el hilo del barbero termine. Al igual que los clientes, el hilo del barbero ejecuta un bucle infinito, por lo que esto no sucederá, pero es una buena práctica esperar a que el hilo termine antes de finalizar el programa.
    barber.join().unwrap();
}