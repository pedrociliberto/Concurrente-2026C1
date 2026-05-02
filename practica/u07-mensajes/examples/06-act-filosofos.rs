extern crate actix;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use actix::{Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, Message, Recipient, System, WrapFuture};
use actix::clock::sleep;
use actix::dev::fut::future::Map;
use actix::fut::result;
use actix_async_handler::async_handler;
use rand::{Rng, thread_rng};

const N: usize = 5;

// El problema de los filosofos comensales, implementado con actores y mensajes. Cada filósofo es un actor que se comunica con sus vecinos para pedir y entregar los palitos (tenedores) necesarios para comer. 
// Se evita el deadlock inicializando el estado de los palitos de manera asimétrica (con la solución de Chandy/Misra), usando la lógica de palitos limpios y sucios.


// Se define Neighbours como un HashMap que asocia cada ChopstickId con la dirección del actor Philosopher que tiene ese palito. Esto permite a cada filósofo enviar mensajes a sus vecinos para pedir o entregar palitos.
type Neighbours = HashMap<ChopstickId, Addr<Philosopher>>;

// Mensaje SetNeighbours para que cada filósofo reciba las direcciones de sus vecinos. Este mensaje se envía al inicio para establecer la comunicación entre los actores.
#[derive(Message)]
#[rtype(result = "()")]
struct SetNeighbours(Neighbours);

// Mensaje Think para indicar que el filósofo está pensando. Este mensaje se envía después de recibir los vecinos y después de terminar de comer.
#[derive(Message)]
#[rtype(result = "()")]
struct Think;

// Mensaje Hungry para indicar que el filósofo tiene hambre y quiere comer. Este mensaje se envía después de un tiempo pensando.
#[derive(Message)]
#[rtype(result = "()")]
struct Hungry;

// Mensaje ChopstickRequest para pedir un palito a un vecino. Este mensaje incluye el ChopstickId del palito que se está pidiendo.
#[derive(Message)]
#[rtype(result = "()")]
struct ChopstickRequest(ChopstickId);

// Mensaje ChopstickResponse para responder a una solicitud de palito. Este mensaje incluye el ChopstickId del palito que se está pidiendo.
#[derive(Message)]
#[rtype(result = "()")]
struct ChopstickResponse(ChopstickId);

// Mensaje TryToEat para que el filósofo intente comer. Este mensaje se envía después de pedir los palitos necesarios.
#[derive(Message)]
#[rtype(result = "()")]
struct TryToEat;

// Mensaje EatingDone para indicar que el filósofo ha terminado de comer. Este mensaje se envía después de un tiempo comiendo.
#[derive(Message)]
#[rtype(result = "()")]
struct EatingDone;

// Enum ChopstickState para representar el estado de cada palito. Un filósofo puede no tener un palito (DontHave), tenerlo sucio (Dirty), tenerlo limpio (Clean) o haberlo solicitado (Requested).
#[derive(PartialEq)]
enum ChopstickState {
    DontHave,
    Dirty,
    Clean,
    Requested
}

// Struct ChopstickId para identificar cada palito. Se utiliza un usize para representar el ID del palito.
#[derive(PartialEq, Eq, Hash, Copy, Clone)]
struct ChopstickId(usize);

// Struct Philosopher para representar a cada filósofo. Cada filósofo tiene un ID, un HashMap que representa el estado de los palitos que necesita, y un HashMap de vecinos para comunicarse con los otros filósofos.
struct Philosopher {
    id: usize,
    chopsticks: HashMap<ChopstickId, ChopstickState>,
    neighbours: Neighbours
}

// Implementación del actor Philosopher. Cada filósofo puede manejar los mensajes definidos anteriormente para realizar las acciones correspondientes (pensar, tener hambre, pedir palitos, responder a solicitudes de palitos, intentar comer y terminar de comer).
impl Actor for Philosopher {
    type Context = Context<Self>;
}

// Implementación del handler para el mensaje SetNeighbours. Cuando un filósofo recibe este mensaje, actualiza su lista de vecinos y comienza a pensar.
impl Handler<SetNeighbours> for Philosopher {
    type Result = ();

    fn handle(&mut self, msg: SetNeighbours, ctx: &mut Context<Self>) -> Self::Result {
        println!("[{}] recibi a mis vecinos", self.id);
        self.neighbours = msg.0;
        // Después de recibir a los vecinos, el filósofo comienza a pensar.
        ctx.address().try_send(Think).unwrap();
    }
}

// Implementación del handler para el mensaje Think. Cuando un filósofo recibe este mensaje, simula el tiempo que pasa pensando y luego envía un mensaje Hungry para indicar que tiene hambre. El Sleep es asíncrono para no bloquear el actor mientras piensa.
#[async_handler]
impl Handler<Think> for Philosopher {
    type Result = ();

    async fn handle(&mut self, msg: Think, ctx: &mut Context<Self>) -> Self::Result {
        println!("[{}] pensando", self.id);
        // Simula el tiempo que el filósofo pasa pensando con un sleep aleatorio entre 2 y 5 segundos.
        sleep(Duration::from_millis(thread_rng().gen_range(2000, 5000))).await;
        // Después de pensar, el filósofo tiene hambre y envía un mensaje Hungry para intentar comer.
        ctx.address().try_send(Hungry).unwrap();
    }
}

// Implementación del handler para el mensaje TryToEat. Cuando un filósofo recibe este mensaje, verifica si tiene todos los palitos necesarios para comer. Si los tiene, simula el tiempo que pasa comiendo y luego envía un mensaje EatingDone. Si no los tiene, simplemente indica que aún no puede comer.
#[async_handler]
impl Handler<TryToEat> for Philosopher {
    type Result = ();

    async fn handle(&mut self, msg: TryToEat, ctx: &mut Context<Self>) -> Self::Result {
        if self.chopsticks.iter().all(|(_id, state)| *state != ChopstickState::DontHave) { // si los tengo todos
            println!("[{}] comiendo", self.id);
            sleep(Duration::from_millis(thread_rng().gen_range(2000, 5000))).await;
            // Después de comer, el filósofo envía un mensaje EatingDone para indicar que ha terminado de comer.
            ctx.address().try_send(EatingDone).unwrap();
        } else {
            println!("[{}] aun no puedo comer", self.id);
        }
    }
}

// Implementación del handler para el mensaje Hungry. Cuando un filósofo recibe este mensaje, verifica qué palitos le faltan y envía mensajes ChopstickRequest a sus vecinos para pedir los palitos que necesita. Luego, envía un mensaje TryToEat para intentar comer.
#[async_handler]
impl Handler<Hungry> for Philosopher {
    type Result = ();

    async fn handle(&mut self, _msg: Hungry, ctx: &mut Context<Self>) -> Self::Result {
        println!("[{}] por comer", self.id);
        // Por cada palito que el filósofo no tiene, envía un mensaje ChopstickRequest a su vecino correspondiente para pedir ese palito.
        for (chopstick_id, state) in self.chopsticks.iter() {
            if *state == ChopstickState::DontHave {
                println!("[{}] pido palito {}", self.id, chopstick_id.0);
                // Envía un mensaje ChopstickRequest al vecino que tiene el palito que se necesita. El mensaje incluye el ID del palito que se está pidiendo.
                self.neighbours.get(chopstick_id).unwrap().try_send(ChopstickRequest(*chopstick_id)).unwrap();
            }
        }
        // Después de pedir los palitos necesarios, el filósofo envía un mensaje TryToEat para intentar comer.
        ctx.address().try_send(TryToEat).unwrap();
    }
}

// Implementación del handler para el mensaje ChopstickRequest. Cuando un filósofo recibe este mensaje, verifica el estado del palito solicitado. Si el palito está sucio, se lo entrega inmediatamente al vecino que lo pidió y actualiza su estado a DontHave. Si el palito está limpio, indica que se lo entregará después de terminar de comer y actualiza su estado a Requested. Si el palito no lo tiene, simplemente indica que no debería pasar (ya que no debería recibir una solicitud por un palito que no tiene).
impl Handler<ChopstickRequest> for Philosopher {
    type Result = ();

    fn handle(&mut self, msg: ChopstickRequest, _ctx: &mut Context<Self>) -> Self::Result {
        println!("[{}] me piden palito {}", self.id, msg.0.0);
        let chopstick = msg.0;
        let chopstick_state = &self.chopsticks.get(&chopstick);
        match chopstick_state {
            Some(ChopstickState::Dirty) => {
                println!("[{}] se lo doy ahora", self.id);
                // Envía un mensaje ChopstickResponse al vecino que pidió el palito, indicando que se lo está entregando. El mensaje incluye el ID del palito que se está entregando.
                self.neighbours.get(&chopstick).unwrap().try_send(ChopstickResponse(msg.0)).unwrap();
                // Después de entregar el palito, se actualiza el estado del palito a DontHave para indicar que ya no lo tiene.
                self.chopsticks.insert(chopstick, ChopstickState::DontHave);
            },
            Some(ChopstickState::Clean) => {
                println!("[{}] se lo doy cuando termine", self.id);
                // Si el palito está limpio, se lo entregará después de terminar de comer. Por lo tanto, se actualiza el estado del palito a Requested para indicar que se ha solicitado y se entregará después de comer.
                self.chopsticks.insert(chopstick, ChopstickState::Requested);
            },
            _ => {
                println!("[{}] no deberia pasar", self.id);
            }
        }
    }
}

// Implementación del handler para el mensaje ChopstickResponse. Cuando un filósofo recibe este mensaje, significa que ha recibido un palito que había solicitado. Por lo tanto, se actualiza el estado del palito a Clean para indicar que ahora lo tiene y está limpio. Luego, se envía un mensaje TryToEat para intentar comer con el nuevo palito que se ha recibido.
#[async_handler]
impl Handler<ChopstickResponse> for Philosopher {
    type Result = ();

    async fn handle(&mut self, msg: ChopstickResponse, ctx: &mut Context<Self>) -> Self::Result {
        println!("[{}] recibi palito {}", self.id, msg.0.0);
        // Después de recibir el palito solicitado, se actualiza el estado del palito a Clean para indicar que ahora lo tiene y está limpio.
        self.chopsticks.insert(msg.0,ChopstickState::Clean);
        // Después de recibir el palito, el filósofo envía un mensaje TryToEat para intentar comer con el nuevo palito que se ha recibido (quizás ya puede o le falta otro, pero de todas formas intenta comer).
        ctx.address().try_send(TryToEat).unwrap();
    }
}

// Implementación del handler para el mensaje EatingDone. Cuando un filósofo recibe este mensaje, significa que ha terminado de comer. Por lo tanto, se revisa el estado de cada palito que tenía. Si un palito estaba solicitado (Requested), se lo entrega al vecino correspondiente y se actualiza su estado a DontHave. Si un palito no estaba solicitado, se marca como sucio (Dirty) para indicar que ahora está sucio después de haber sido usado para comer. Finalmente, el filósofo envía un mensaje Think para volver a pensar después de terminar de comer.
#[async_handler]
impl Handler<EatingDone> for Philosopher {
    type Result = ();

    async fn handle(&mut self, _msg: EatingDone, ctx: &mut Context<Self>) -> Self::Result {
        println!("[{}] terminé de comer", self.id);
        for (chopstick, mut state) in self.chopsticks.iter_mut() {
            if *state == ChopstickState::Requested {
                println!("[{}] entrego palito {}", self.id, chopstick.0);
                // Envía un mensaje ChopstickResponse al vecino que solicitó el palito, indicando que se lo está entregando. El mensaje incluye el ID del palito que se está entregando.
                self.neighbours.get(chopstick).unwrap().try_send(ChopstickResponse(*chopstick)).unwrap();
                *state = ChopstickState::DontHave
            } else {
                println!("[{}] marco como sucio palito {}", self.id, chopstick.0);
                // Si el palito no estaba solicitado, se marca como sucio (Dirty) para indicar que ahora está sucio después de haber sido usado para comer.
                *state = ChopstickState::Dirty
            }
        }
        // Después de terminar de comer y manejar el estado de los palitos, el filósofo envía un mensaje Think para volver a pensar.
        ctx.address().try_send(Think).unwrap();
    }
}

fn main() {
    let system = System::new();
    system.block_on(async {
        let mut philosophers = vec!();

        for id in 0..N {
            // Deadlock avoidance forcing the initial state
            philosophers.push(Philosopher {
                id,
                chopsticks: HashMap::from([
                    (ChopstickId(id), if id == 0 { ChopstickState::Dirty } else { ChopstickState::DontHave }),
                    (ChopstickId((id + 1) % N), if id == N-1 { ChopstickState::DontHave } else { ChopstickState::Dirty })
                ]),
                neighbours: HashMap::with_capacity(2)
            }.start())
        }


        for id in 0..N {
            let prev = if id == 0 { N - 1 } else { id - 1 };
            let next = (id + 1) % N;
            // Se envía un mensaje SetNeighbours a cada filósofo para establecer las direcciones de sus vecinos. Cada filósofo recibe un HashMap que asocia cada ChopstickId con la dirección del actor Philosopher que tiene ese palito. Esto permite a cada filósofo enviar mensajes a sus vecinos para pedir o entregar palitos.
            philosophers[id].try_send(SetNeighbours(HashMap::from([
                (ChopstickId(id), philosophers[prev].clone()),
                (ChopstickId(next), philosophers[next].clone())
            ]))).unwrap();
        }
    });

    system.run().unwrap();
}