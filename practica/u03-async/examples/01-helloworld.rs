use std::time::Duration;
use async_std::task;

/*
1er ejemplo Programación Asíncrona: Hello World.
En este ejemplo, se definen dos funciones asíncronas: `hello` y `world`. La función `hello` simula una operación que tarda 2 segundos en completarse, mientras que la función `world` devuelve inmediatamente una cadena de texto.
*/

// Al ser async, la función no devuelve directamente String, sino un Future que eventualmente resolverá a String. El código dentro de la función se ejecutará cuando se llame a .await en el Future (que es devuelto por la función).

async fn hello() -> String {
    task::sleep(Duration::from_secs(2)).await;
    String::from("Hello")
}

async fn world() -> String {
    String::from(" World!")
}

async fn async_main() -> String {
    println!("Started!");
    // Se crean los objetos Future para cada función asíncrona. Estos objetos representan la operación que se ejecutará en el futuro, pero no se ejecutan inmediatamente.
    let hello_future = hello();
    let world_future = world();
    // Aquí es donde realmente se ejecutan las funciones asíncronas. Al llamar a .await en cada Future, el programa esperará a que cada operación se complete antes de continuar. En este caso, `hello` tardará 2 segundos en completarse, mientras que `world` se resolverá inmediatamente.
    let hello_result = hello_future.await;
    let world_result = world_future.await;
    hello_result + world_result.as_str()
}

fn main() {
    // block_on es el puente entre el código sincrónico y el código asíncrono. Permite ejecutar una función asíncrona desde un contexto sincrónico, bloqueando el hilo principal hasta que la función asíncrona se complete. En este caso, se ejecuta `async_main`, que a su vez llama a las funciones `hello` y `world`. El resultado final se imprime en la consola.
    println!("{}", task::block_on(async_main()));
}