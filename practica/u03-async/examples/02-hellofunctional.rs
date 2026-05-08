use std::future::Future;
use std::time::Duration;

use async_std::task;
use futures::FutureExt;

async fn hello() -> String {
    task::sleep(Duration::from_secs(2)).await;
    String::from("Hello")
}

async fn world() -> String {
    String::from(" World!")
}

// Aquí no usamos la palabra clave async en la definición de la función. En su lugar, decimos explícitamente que esta función devuelve algo que implementa el rasgo Future y que, cuando termine, entregará un String.
fn async_main() -> impl Future<Output=String> {
    hello()
        .map(|h| world().map(|w| h + w.as_str()))
        .flatten()
}

fn main() {
    println!("{}", task::block_on(async_main()));
}