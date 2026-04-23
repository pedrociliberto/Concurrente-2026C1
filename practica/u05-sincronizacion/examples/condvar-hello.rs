use std::sync::{Arc, Mutex, Condvar};
use std::thread;
use std::time::Duration;

/*
Usamos CONDVARS para sincronizar a dos hilos, uno que hace un cálculo costoso y luego notifica al otro hilo que el resultado está listo. El hilo que espera se bloquea hasta que el hilo que hace el cálculo notifique que el resultado está listo. 
Mientras la Condvar tenga un valor de true, el hilo que espera seguirá bloqueado. Cuando el hilo que hace el cálculo termine, cambiará el valor a false y notificará al hilo que espera, que entonces podrá continuar con su ejecución.
*/

fn main() {

    // Creamos un par de Mutex y Condvar para sincronizar a los hilos. El Mutex protege el acceso a un valor booleano que indica si el resultado del cálculo costoso está listo o no. La Condvar se utiliza para bloquear al hilo que espera hasta que el resultado esté listo.
    let pair = Arc::new((Mutex::new(true), Condvar::new()));

    // Se clona el Arc para pasarlo al hilo que hace el cálculo costoso. Esto permite que ambos hilos compartan el mismo par de Mutex y Condvar.
    let pair_clone = pair.clone();
    thread::spawn(move || {
        let (lock, cvar) = &*pair_clone;

        // Se simula un spurious wakeup, donde el hilo que espera podría despertar sin que el hilo que hace el cálculo haya notificado. Esto se hace para demostrar que el hilo que espera debe verificar la condición después de despertar, en lugar de asumir que el resultado está listo.
        cvar.notify_all(); // spurious wakeup example

        // Se hace el cálculo costoso (simulado con 1 segundo de pausa).
        println!("[awaited] doing expensive computation");
        thread::sleep(Duration::from_millis(1000));
        // Termina, y lockea el mutex para cambiar el valor a false, indicando que el resultado está listo.
        println!("[awaited] done");
        let mut pending = lock.lock().unwrap(); // Se obtiene la Condvar a partir de lockear el mutex.
        println!("[awaited] got lock");
        *pending = false;

        // Notifica al hilo que espera que el resultado está listo. Esto despertará al hilo que espera, que entonces podrá verificar la condición y continuar con su ejecución.
        println!("[awaited] notifying");
        cvar.notify_all();
    });


    let (lock, cvar) = &*pair;

    // El hilo principal se bloquea en la Condvar, esperando a que el resultado del cálculo costoso esté listo. La función wait_while se utiliza para bloquear al hilo hasta que la condición especificada (en este caso, que el valor booleano sea false) se cumpla. Si el hilo despierta debido a un spurious wakeup, volverá a verificar la condición y seguirá bloqueado si el resultado aún no está listo.
    let _guard = cvar.wait_while(lock.lock().unwrap(), |pending| {
        println!("[waiter] checking condition {}", *pending);
        *pending
    }).unwrap();

    println!("[waiter] current mutex content {}", *_guard);
    println!("[waiter] done");

}
