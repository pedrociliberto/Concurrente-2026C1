extern crate actix;

use std::collections::HashSet;
use actix::{Actor, Context, Handler, System, Message, AsyncContext, Recipient, ActorFutureExt};
use rand::{thread_rng, Rng};
use actix::clock::sleep;
use std::time::Duration;
use actix_async_handler::async_handler;

const WORKERS:usize = 5;

// Process es un mensaje que el coordinador envía a los workers para que procesen una cantidad de señal y devuelvan un resultado. 
#[derive(Message)]
#[rtype(result = "()")]
struct Process {
    amount: f64, // cantidad de señal a procesar
    sender: Recipient<Result> // dirección del coordinador para que el worker le devuelva el resultado
}

// Result es un mensaje que los workers envían al coordinador para devolver el resultado de procesar la señal. El primer campo es el id del worker que envía el resultado, y el segundo campo es el resultado del procesamiento (su señal final).
#[derive(Message)]
#[rtype(result = "()")]
struct Result(usize, f64);

// Epoch es un mensaje que el coordinador se envía a sí mismo para iniciar una nueva epoch de procesamiento. El campo es la cantidad de señal total que se debe procesar en esa epoch.
#[derive(Message, Debug)]
#[rtype(result = "()")]
struct Epoch(f64);

// El coordinador es el actor que coordina el procesamiento de la señal. Recibe un mensaje Epoch para iniciar una nueva epoch, y envía un mensaje Process a cada worker para que procesen su parte de la señal. Luego, recibe mensajes Result de los workers con los resultados parciales, y cuando recibe todos los resultados, calcula el resultado final y se envía un nuevo mensaje Epoch para iniciar la siguiente epoch.
struct Coordinator {
    signal: f64,
    workers: Vec<Recipient<Process>>,
    results: HashSet<usize>
}

// El worker es el actor que procesa la señal. Recibe un mensaje Process con la cantidad de señal a procesar, simula un procesamiento (con un sleep aleatorio), y luego envía un mensaje Result al coordinador con su resultado. Tiene un id para identificarlo en los resultados que envía al coordinador.
struct Worker {
    id: usize,
}

// Implementamos los actores para cada uno de ellos (coordinador y trabajador).

impl Actor for Coordinator {
    type Context = Context<Self>;
}

impl Actor for Worker {
    type Context = Context<Self>;
}

// El handler del coordinador para el mensaje Epoch es síncrono porque simplemente divide la señal entre los workers, envía los mensajes Process a cada uno, y luego espera a recibir los resultados. No hay ningún procesamiento que requiera esperar o hacer algo asíncrono dentro de este handler, por lo que no es necesario que sea asíncrono.
impl Handler<Epoch> for Coordinator {
    type Result = ();

    // Divide la señal total entre el número de workers para calcular la cantidad de señal que cada worker debe procesar. Luego, envía un mensaje Process a cada worker con esa cantidad de señal y la dirección del coordinador para que le devuelvan el resultado. También reinicia la señal total y el conjunto de resultados para la nueva epoch.
    fn handle(&mut self, _msg: Epoch, _ctx: &mut Context<Self>) -> Self::Result {
        let signal = _msg.0;
        let signal_local = signal / self.workers.len() as f64;
        self.signal = 0.;
        self.results.clear();

        println!("[COORDINADOR] empieza con señal {}", signal);

        for worker in self.workers.iter() {
            worker.try_send(Process { amount: signal_local, sender: _ctx.address().recipient()}).unwrap();
        }


    }
}

// El handler del coordinador para el mensaje Result es síncrono porque simplemente recibe el resultado de un worker, lo acumula en la señal total, y cuando recibe todos los resultados, calcula el resultado final y se envía un nuevo mensaje Epoch para iniciar la siguiente epoch. No hay ningún procesamiento que requiera esperar o hacer algo asíncrono dentro de este handler, por lo que no es necesario que sea asíncrono.
impl Handler<Result> for Coordinator {
    type Result = ();

    // Recibe el resultado de un worker, lo acumula en la señal total, y cuando recibe todos los resultados, calcula el resultado final y se envía un nuevo mensaje Epoch para iniciar la siguiente epoch. También verifica que no se dupliquen los resultados de los workers, ya que cada worker solo debe enviar un resultado por epoch. Si recibe un resultado de un worker que ya ha enviado su resultado, simplemente lo ignora.
    fn handle(&mut self, msg: Result, _ctx: &mut Context<Self>) -> Self::Result {

        println!("[COORDINADOR] recibí resultado de worker {}", msg.0);

        if !self.results.contains(&msg.0) {
            self.signal += msg.1;
            self.results.insert(msg.0);

            if self.results.len() == self.workers.len() {
                println!("[COORDINADOR] fin de la epoch, resultado final {}", self.signal);
                _ctx.address().try_send(Epoch(self.signal)).unwrap();
            }
        }

    }
}

// El handler del worker es asíncrono porque simula un procesamiento que tarda un tiempo aleatorio (con un sleep), y luego envía el resultado al coordinador. Por eso usamos el atributo #[async_handler] para indicar que el handler es asíncrono, y podemos usar await dentro de él.
#[async_handler]
impl Handler<Process> for Worker {
    type Result = ();

    fn handle(&mut self, msg: Process, _ctx: &mut Context<Self>) -> Self::Result {
        println!("[WORKER {}] recibo {}", self.id, msg.amount);
        sleep(Duration::from_millis(thread_rng().gen_range(500, 1500))).await;
        let resultado = msg.amount * thread_rng().gen_range(0., 1.);
        println!("[WORKER {}] devuelvo {}", self.id, resultado);
        msg.sender.try_send(Result(self.id, resultado)).unwrap();
    }
}

fn main() {
    let system = System::new();
    system.block_on(async {
        let mut workers = vec!();

        // .recipient() es un método que se llama sobre la dirección de un actor para obtener una dirección que solo puede recibir mensajes de un tipo específico (en este caso, Process). Esto es útil para enviar mensajes a los workers sin exponer toda su dirección, lo que mejora la encapsulación y seguridad del sistema de actores. Al usar recipient(), el coordinador solo puede enviar mensajes de tipo Process a los workers, y no puede enviarles otros tipos de mensajes o interactuar con ellos de otras formas.
        for id in 0..WORKERS {
            workers.push(Worker { id }.start().recipient())
        }

        // Instanciamos el coordinador con la lista de direcciones de los workers, y le enviamos un mensaje Epoch para iniciar la primera epoch de procesamiento con una cantidad de señal inicial (en este caso, 1000.0). El coordinador se encargará de dividir esa señal entre los workers, esperar sus resultados, y luego iniciar la siguiente epoch con el resultado final.
        Coordinator { signal: 0.0, workers, results: HashSet::with_capacity(WORKERS) }.start()
            .do_send(Epoch(1000.0));
    });

    system.run().unwrap();

}