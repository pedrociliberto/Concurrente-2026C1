/// Ejemplo inicial de uso de hilos en Rust. Este programa crea 10 hilos, cada uno de los cuales imprime un mensaje indicando que está ejecutándose. Luego, el programa espera a que todos los hilos terminen antes de finalizar.
pub fn main() {
    let mut handles = vec![]; // `handles` es un vector que almacenará los `JoinHandle<()>` de cada hilo creado. Un `JoinHandle` es un tipo que representa un hilo de ejecución y permite esperar a que el hilo termine su ejecución.

    for i in 0..10 {
        // `handle` es una variable de tipo `JoinHandle<()>`, que representa un hilo de ejecución.
        // `std::thread::spawn` se utiliza para crear un nuevo hilo de ejecución. El bloque de código dentro del `move || { ... }` es lo que se ejecutará en ese hilo.
        // El `move` es necesario para que el hilo pueda tomar posesión de las variables que se utilizan dentro del bloque, en este caso, la variable `i`.
        // || es la sintaxis para una función anónima (closure) que no toma argumentos y devuelve `()`.
        // Si la función tomara argumentos, se colocarían dentro de los paréntesis después de `||`.
        let handle = std::thread::spawn(move || {
            println!("Thread {} is running", i);
        });
        handles.push(handle); // El `handle` del hilo recién creado se agrega al vector `handles` para que podamos esperar a que termine más adelante.
    }

    // Después de crear todos los hilos, se itera sobre el vector `handles` y se llama a `join()` en cada uno de ellos.
    for handle in handles {
        // `join()` es un método que bloquea el hilo actual hasta que el hilo representado por `handle` termine su ejecución. Si el hilo termina con éxito, `join()` devuelve `Ok(())`. Si el hilo termina con un error, devuelve `Err(e)`, donde `e` es el error que causó la terminación del hilo.
        handle.join().unwrap(); // `unwrap()` es una mala práctica, ya que puede causar un pánico si el hilo termina con un error. Debería manejar el error acordemente.
    }

    // Si ejecutamos este programa, veremos que se crean 10 hilos, cada uno imprimiendo un mensaje con su número de hilo. El programa esperará a que todos los hilos terminen antes de finalizar.
    // Sin embargo, el orden en que los mensajes se imprimen puede variar entre ejecuciones, ya que los hilos se ejecutan de manera concurrente y no hay garantía de que se ejecuten en un orden específico.

    // Una de las ejecuciones me dio así:

    // Thread 3 is running
    // Thread 2 is running
    // Thread 0 is running
    // Thread 1 is running
    // Thread 4 is running
    // Thread 5 is running
    // Thread 6 is running
    // Thread 8 is running
    // Thread 7 is running
    // Thread 9 is running
}

