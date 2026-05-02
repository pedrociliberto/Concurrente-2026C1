extern crate actix;

use actix::{Actor, Context, Handler, System, Message};

// En este ejemplo, tenemos una calculadora (Calc) que tiene un estado interno (current), y que puede manejar mensajes de tipo Add y Sub para actualizar su estado interno. La calculadora es un actor, y cada vez que recibe un mensaje de tipo Add o Sub, actualiza su estado interno y devuelve el nuevo valor de current. De esta manera, la calculadora mantiene un estado interno que se va actualizando con cada mensaje que recibe.

#[derive(Message)]
#[rtype(result = "i32")]
struct Add(i32);

#[derive(Message)]
#[rtype(result = "i32")]
struct Sub(i32);

// Tenemos una calculadora (Calc) que tiene un estado interno (el valor actual de la calculadora).
struct Calc {
    current: i32
}

// La calculadora (Calc) es un actor, y puede manejar mensajes de tipo Add y Sub. Al manejar un mensaje de tipo Add, la calculadora suma el valor del mensaje a su estado interno (current), y devuelve el nuevo valor de current. Al manejar un mensaje de tipo Sub, la calculadora resta el valor del mensaje a su estado interno (current), y devuelve el nuevo valor de current. De esta manera, la calculadora mantiene un estado interno que se va actualizando con cada mensaje que recibe.
impl Actor for Calc {
    type Context = Context<Self>;
}

// Implementamos un Handler para el tipo de mensaje Add, para el actor Calc. Esto significa que el actor Calc puede manejar mensajes de tipo Add, y que al manejar ese mensaje, se espera un resultado de tipo i32 (como se especificó en la implementación del trait Message para Add).
impl Handler<Add> for Calc {
    type Result = i32;

    fn handle(&mut self, msg: Add, _ctx: &mut Context<Self>) -> Self::Result {
        println!("add {}", msg.0);
        self.current += msg.0;
        self.current
    }
}

// Implementamos un Handler para el tipo de mensaje Sub, para el actor Calc. Esto significa que el actor Calc puede manejar mensajes de tipo Sub, y que al manejar ese mensaje, se espera un resultado de tipo i32 (como se especificó en la implementación del trait Message para Sub).
impl Handler<Sub> for Calc {
    type Result = i32;

    fn handle(&mut self, msg: Sub, _ctx: &mut Context<Self>) -> Self::Result {
        println!("sub {}", msg.0);
        self.current -= msg.0;
        self.current
    }
}

#[actix_rt::main]
async fn main() {
    let addr = Calc { current: 0 }.start(); 

    // Le manda un mensaje al Addr de la calculadora, pero no espera a que se resuelva el futuro para obtener la respuesta. Es un mensaje de tipo Add con el valor 20, y la calculadora actualizará su estado interno (current) sumando 20 al valor actual. Como no esperamos a que se resuelva el futuro, no obtenemos la respuesta de la calculadora, pero el mensaje se procesa y el estado interno de la calculadora se actualiza.
    addr.do_send(Add(20));
    println!("do_send done");

    // Le manda un mensaje sin esperar, pero en este caso es try_send, por lo qu (por ejemplo) si el mailbox del actor está lleno, el mensaje no se envía y se devuelve un error. En este caso, como el mailbox no está lleno, el mensaje se envía correctamente y la calculadora actualiza su estado interno sumando 15 al valor actual. Al igual que con do_send, no esperamos a que se resuelva el futuro para obtener la respuesta, pero el mensaje se procesa y el estado interno de la calculadora se actualiza.
    addr.try_send(Add(15)).unwrap();
    println!("try_send done");

    // En este sí aguardamos al resultado del envío del mensaje.
    let res = addr.send(Add(5)).await;
    println!("Calc: {}", res.unwrap());

    // Acá también. El mensaje es de tipo Sub con el valor 3, y la calculadora actualizará su estado interno restando 3 al valor actual. Al esperar a que se resuelva el futuro, obtenemos la respuesta de la calculadora, que es el nuevo valor de current después de restar 3.
    let res = addr.send(Sub(3)).await;

    println!("Calc: {}", res.unwrap());
    System::current().stop();
}
