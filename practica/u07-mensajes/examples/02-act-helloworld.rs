extern crate actix;

use actix::{Actor, Context, Handler, System, Message};
// Actix se monta sobre Tokio, que es un runtime de Rust para programación asíncrona. Actix utiliza Tokio para manejar la concurrencia y el paso de mensajes entre actores.
// Es el middleware para que cada actor tenga su propio mailbox, y para que el paso de mensajes entre actores sea eficiente y seguro.

/*
Los ACTORES son una abstracción de concurrencia que se basa en el paso de mensajes. 
Un actor es una entidad que tiene un estado interno, y que puede recibir mensajes y enviar mensajes a otros actores.
Cada actor tiene su propio hilo de ejecución, y no comparte estado con otros actores. 
Los actores pueden ser creados y destruidos dinámicamente, y pueden comunicarse entre sí a través de mensajes.

Se definen por los tipos de mensaje que pueden recibir, y por el comportamiento que tienen al recibir esos mensajes.
*/

// Para que un tipo (en este caso SayHello) pueda ser utilizado como mensaje en Actix, debe implementar el trait Message, y debe especificar el tipo de resultado que se espera al manejar ese mensaje. En este caso, el mensaje SayHello espera un resultado de tipo String. 
// En este caso, la respuesta automática cuando se procesa el mensaje es de tipo String. No tiene por qué ser el mismo tipo que el envío del mensaje.

// Puedo usar estas 2 líneas comentadas para hacerlo más rápido, pero es más explícito hacerlo como está hecho en el código.

//#[derive(Message)]
//#[rtype(result = "String")]
struct SayHello {
    name: String
}


impl Message for SayHello {
    type Result = String;
}

// Este Struct será el actor que va a manejar los mensajes de tipo SayHello. El actor Greeter no tiene ningún estado interno, pero podría tenerlo si lo necesitara.
struct Greeter {

}

// Para que un tipo (en este caso Greeter) pueda ser utilizado como actor en Actix, debe implementar el trait Actor, y debe especificar el tipo de contexto que utiliza. En este caso, el actor Greeter utiliza el contexto Context<Self>, que es el contexto por defecto para los actores en Actix. El contexto es el entorno de ejecución del actor, y se encarga de manejar la cola de mensajes y el ciclo de vida del actor.
impl Actor for Greeter {
    type Context = Context<Self>;
}

// Implemento un Handler para el tipo de mensaje SayHello, para el actor Greeter. Esto significa que el actor Greeter puede manejar mensajes de tipo SayHello, y que al manejar ese mensaje, se espera un resultado de tipo String (como se especificó en la implementación del trait Message para SayHello).
impl Handler<SayHello> for Greeter {
    // El tipo de resultado que se espera al manejar un mensaje de tipo SayHello es String, como se especificó en la implementación del trait Message para SayHello. Por eso, el tipo de resultado de este Handler es String.
    type Result = String;

    // El método handle es el que se llama cuando el actor recibe un mensaje de tipo SayHello, y es donde se define el comportamiento del actor al recibir ese mensaje. En este caso, el actor Greeter responde con un saludo que incluye el nombre que se le pasó en el mensaje.
    fn handle(&mut self, msg: SayHello, _ctx: &mut Context<Self>) -> Self::Result {
        "Hello ".to_owned() + &msg.name
    }
}

// Corre la aplicación en un sistema configurado para usar Actix. El sistema es el entorno de ejecución de Actix, y se encarga de manejar los actores y el paso de mensajes entre ellos.
#[actix_rt::main]
async fn main() {
    // Instanciamos un actor de tipo Greeter, y obtenemos su dirección (addr) para poder enviarle mensajes. El método start() crea una nueva instancia del actor y lo inicia, devolviendo su dirección.
    let addr = Greeter {}.start();

    // Al mailbox (addr) le podemos enviar mensajes utilizando el método send(), que devuelve un futuro que se resuelve con el resultado del manejo del mensaje. En este caso, le enviamos un mensaje de tipo SayHello con el nombre "world!", y esperamos a que se resuelva el futuro para obtener la respuesta. Usamos .await para esperar a que se resuelva el futuro.
    let res = addr.send(SayHello { name: String::from("world!") }).await;

    println!("{}", res.unwrap());

    // Detenemos el sistema de Actix. Esto es necesario para que la aplicación termine, ya que el sistema de Actix se ejecuta en un bucle infinito esperando mensajes. Si no detenemos el sistema, el sistema de actores seguirá ejecutándose y la aplicación no terminará. Al llamar a System::current().stop(), estamos indicando que queremos detener el sistema de Actix, lo que hará que la aplicación termine.
    System::current().stop();
}