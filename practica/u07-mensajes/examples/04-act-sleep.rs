extern crate actix;

use std::{thread, io};
use std::time::{Duration, SystemTime};

use actix::{Actor, ActorFutureExt, Context, Handler, Message, SyncArbiter, System, SyncContext};
use std::io::Read;
use actix_async_handler::async_handler;

#[derive(Message)]
#[rtype(result = "()")]
struct Sleep(u64);

struct Sleepyhead {
    id: usize
}

// Tenemos un actor "dormilón". Le pasamos cuánto tiempo tiene que dormir.
impl Actor for Sleepyhead {
    type Context = Context<Self>; //SyncContext<Self>;
}

// Definimos un Handler para el mensaje Sleep, que es un mensaje que le dice al actor cuánto tiempo tiene que dormir. En este caso, el Handler es asíncrono, lo que significa que puede realizar operaciones asíncronas (como dormir) sin bloquear el hilo de ejecución del actor. Al manejar un mensaje de tipo Sleep, el actor imprimirá un mensaje indicando que está durmiendo por la cantidad de segundos especificada en el mensaje, luego dormirá durante ese tiempo utilizando tokio::time::sleep, y finalmente imprimirá un mensaje indicando que se despertó.
#[async_handler]
impl Handler<Sleep> for Sleepyhead {
    type Result = ();

    async fn handle(&mut self, msg: Sleep, _ctx: &mut <Sleepyhead as Actor>::Context) -> Self::Result  {
        println!("[{}] durmiendo por {}", self.id, msg.0);
        // Duerme durante la cantidad de segundos especificada en el mensaje, sin bloquear el hilo de ejecución del actor. Esto permite que el actor pueda manejar otros mensajes mientras está "durmiendo".
        tokio::time::sleep(Duration::from_secs(msg.0)).await;
        // thread::sleep(Duration::from_secs(msg.0));
        println!("[{}] desperté de {}", self.id, msg.0);
    }
}

// Si en lugar de usar un Handler asíncrono, usáramos un Handler síncrono que utiliza thread::sleep para dormir, el actor se bloquearía durante el tiempo que está durmiendo, lo que significa que no podría manejar otros mensajes mientras está "durmiendo". Esto haría que el actor sea menos eficiente y menos capaz de manejar múltiples tareas al mismo tiempo.
// El sleep de Tokio es asíncrono, lo que significa que no bloquea el hilo de ejecución del sistema general mientras está "durmiendo". Esto permite que otros actores puedan manejar otros mensajes mientras está "durmiendo". En cambio, el sleep de std::thread es síncrono, lo que significa que bloquea el hilo de ejecución del sistema general mientras está "durmiendo".
// El orden de los mensajes para un mismo actor sí se respeta, pero como cada actor tiene su propio hilo de ejecución, el orden de los mensajes entre diferentes actores no se garantiza.

// También podemos tener un Handler sincrónico, usando SyncArbitrer. En este caso, cada actor se ejecuta en un hilo separado, y el Handler de Sleep utiliza thread::sleep para dormir. En este caso, el actor se bloqueará durante el tiempo que está durmiendo, lo que significa que no podrá manejar otros mensajes mientras está "durmiendo". Pero entre actores no hay problema porque viven en su propio hilo, entonces el actor 1 puede estar durmiendo mientras el actor 2 maneja otros mensajes sin problemas. 
impl Handler<Sleep> for Sleepyhead {
    type Result = ();

    fn handle(&mut self, msg: Sleep, _ctx: &mut <Sleepyhead as Actor>::Context) -> Self::Result  {
        println!("[{}] durmiendo por {}", self.id, msg.0);
        thread::sleep(Duration::from_secs(msg.0 * 10));
        println!("[{}] desperté de {}", self.id, msg.0);
    }
}

#[actix_rt::main]
async fn main() {
    // console_subscriber::init();

    println!("Enter para empezar");
    io::stdin().read(&mut [0u8]).unwrap();

    // Se crean dos actores "dormilones" con diferentes IDs. Estos actores pueden manejar mensajes de tipo Sleep, y cada uno tiene su propio estado (en este caso, su ID) que se utiliza para identificarlo en los mensajes que imprime al dormir y despertar.

    let addr = Sleepyhead { id: 1 }.start(); 

    // Podríamos usar SyncArbiter para crear un actor que se ejecute en un hilo separado, lo que permitiría que el Handler de Sleep utilice thread::sleep para dormir sin bloquear el hilo principal del programa.
    // let addr = SyncArbiter::start(1, || Sleepyhead { id: 1 });
    
    // También podríamos iniciarlo con un valor mayor a 1 (por ejemplo, 2) para crear múltiples instancias del mismo actor (cada una con su estado independiente), lo que le permitiría manejar múltiples mensajes de tipo Sleep al mismo tiempo sin bloquearse y esperar a que termine para recibir otros mensajes.
    // let addr = SyncArbiter::start(2, || Sleepyhead { id: 1 });

    let other = Sleepyhead { id: 2 }.start(); 
    // let other = SyncArbiter::start(1, || Sleepyhead { id: 2 });

    let now = SystemTime::now();

    // Ambos actores reciben mensajes de tipo Sleep con diferentes duraciones con try_send. Debido a que el Handler de Sleep es asíncrono, ambos actores pueden manejar sus mensajes de sueño sin bloquearse mutuamente, lo que significa que el segundo actor se despertará antes que el primero, a pesar de haber recibido su mensaje después del primero.

    addr.try_send(Sleep(3)).unwrap();
    println!("mandé 3 al 1");

    other.try_send(Sleep(2)).unwrap();
    println!("mandé 2 al 2");

    // Acá sí se espera a que el primer actor termine de dormir, lo que significa que el programa no terminará hasta que ambos actores hayan terminado de dormir y se hayan despertado. El tiempo total que tardará el programa en terminar será aproximadamente el tiempo que tarda el actor con ID 1 en dormir (3 segundos) más el tiempo que tarda el actor con ID 2 en dormir (2 segundos), ya que ambos actores pueden dormir al mismo tiempo sin bloquearse mutuamente.
    addr.send(Sleep(2)).await.unwrap();

    println!("terminé. tardé {}", now.elapsed().unwrap().as_secs());


    println!("Enter para terminar");
    io::stdin().read(&mut [0u8]).unwrap();

    System::current().stop();
}