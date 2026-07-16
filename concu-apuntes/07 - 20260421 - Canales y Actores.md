## Clase 7 - Canales y Actores

### Pasaje de mensajes

#### Modelos de comunicación

- **Tipos de Comunicación** (génerico):
	- **Sincrónica**: se está en sincronización temporal entre emisor y receptor.
	- **Asincrónica**: el emisor envía un mensaje y no espera inmediatamente la respuesta. Se acumulan en un *buffer* hasta que el receptor pueda procesarlos.
- **Direccionamiento**: ¿cómo se determina a quién dirigir un mensaje?
	- **Simétrico**: es de un emisor a un receptor (1 a 1)
	- **Asimétrico**: es de un emisor a múltiples destinatarios (grupo, muchas direcciones)
	- **Sin direccionamiento**: no se envía a alguien particular. Se coloca en un *buffer* al cual acceden múltiples receptores. Cada uno lo recibe de acuerdo a si le corresponde o no recibirlo (por la estructura del mensaje).
- **Flujo de datos**:
	- **Unidireccional**: permite enviar durante toda la vida del canal (del emisor al receptor)
	- **Bidireccional**: permite escribir y enviar mensajes en un sentido o en el otro, o de forma alternada.

### Canales

Herramienta de **IPC** (Inter Process Communication).

- Conectan un proceso emisor con otro receptor.
- Tienen un nombre.
- Son tipados.
- Pueden ser **sincrónicos** o **asincrónicos**.
- Son **unidireccionales**.

##### Productor y Consumidor

![[Canales Productores y Consumidores.png]]

- El **productor**, en loop, produce un elemento `I`, y lo inserta en el canal `ch`.
- El **consumidor**, también en loop, extrae del canal el elemento `I`, y luego lo consume. 
- El tipo de dato producido y consumido **es el mismo**.
- El canal provee **sincronismo**: si está vacío, el **consumidor esperará** a que haya un elemento a extraer.
- Cuando el canal está lleno, el **productor se bloquea** hasta que haya espacio disponible en el *buffer* del canal.

#### Service Input

- Sintaxis permitida por los lenguajes que soportan canales.
- Permite escuchar en varios canales de **forma bloqueante** y desbloquearse con en primero que recibe un mensaje.

#### Remote Procedure Calls

Permiten al cliente **ejecutar funciones en un servidor localizado en otro procesador**.

- Se requiere implementación de *stubs* en ambos extremos.
- Los *stubs* conforman **interfaces remotas** utilizadas para compilar cliente y servidor. 
- Internamente llaman al servidor, realizando `marshall` y `unmarshall` con los parámetros (convertirlos a formatos comunes de *bits*).
- El servidor es el que ejecuta el código.
- Ofrecen localización de servicios: consultar cuál es el directorio de servicios.

### Canales en Unix

Pensados para ejecutar con *scripting* de Bash o desde la Shell.

- Provee Pipes y FIFOs para **conectar dos procesos independientes**, orientados a *bytes* (es la mínima unidad enviable)
	- Los FIFOs poseen una representación en el *filesystem*. Se pueden crear y se verán con `ls` normalmente (eso sí, con una letra distinta).
	- Los Pipes se conocen como *Unnamed Pipes*, no se visualizan, sino que yacen simplemente en memoria.
- También provee **colas de mensajes** (*Message Queues*) orientados a tratar a los **mensajes como unidades independientes** (no se pisan otros en caso de leer de más). Permite enviarlos con tipo de dato (estructuras más complejas que tiras de *bytes*). Son sin nombre.
### Canales en Rust

- Tiene dos extremos: un emisor y un receptor. 
- Un thread **ejecuta los envíos desde un extremo** (invoca métodos sobre el transmisor), y otro chequea el **extremo de recepción por la existencia de mensajes** (se bloquea esperando).
- Son ***MPSC***: múltiples productores, un solo consumidor. Si se quiere crear múltiples productores, se clona el extremo de envío, pasándoselo a muchos threads.
- Transfiere el *ownership* del elemento enviado.

```rust
use std::sync::mpsc; 
use std::thread; 

fn main() { 
	let (tx, rx) = mpsc::channel();
	thread::spawn(move || {
		let val = String::from("Hola");
		tx.send(val).unwrap(); // #1
	});
	let received = rx.recv().unwrap(); // #2
	println("Recibido: {}", received);
}
```

- `tx` es el **extremo de transmisión**; `rx` es el **extremo de recepción**.
- El **transmisor** utiliza el método `tx.send(val)` (**#1**). Aquí se traslada el *ownership* del valor enviado.
- El **receptor** utiliza el método `let val = rx.recv()` (**#2**). El *ownership* del valor ya es del receptor en este punto.
### Modelo de actores

- Similar al modelo de pasaje de mensajes.
- Diseñado inicialmente a principios de los años 70.
- Desarrollado por Carl Hewitt en 1973.
- Popularizado por el lenguaje Erlang. 

#### Actores

**Actor**: primitiva principal del modelo.

- Son livianos, se pueden crear miles de ellos (en lugar de threads).
- Internamente los usaremos con **programación asincrónica**.
- Encapsulan comportamiento y estado (solo lo puede acceder él mismo).
- Está compuesto por: 
	- **Dirección**: a donde enviarle mensajes.
	- **Casilla de correo (*mailbox*)**: un FIFO de los últimos mensajes recibidos.
- El actor **supervisor** puede crear otros actores "hijo".

![[Actores Estructura.png]]

- El modelo es parecido al **modelo de Objetos**: los métodos reciben parámetros que pueden ser los mensajes. 
- Cada actor es aislado de otros actores: no comparten memoria.
- El estado privado solo puede cambiarse a partir de **procesar mensajes**.
- Pueden manejar/procesar **un mensaje por vez**.
- En un sistema distribuido, la dirección del actor puede **ser remota**.

#### Mensajes

- Los actores se comunican **solamente usando mensajes**.
- Los mensajes son **procesados** por los actores de forma **asincrónica**: el actor receptor procesará un mensaje cuando le corresponda (y pueda).
- Son estructuras **simples inmutables**: no estarán modificando la memoria, sino copiando información.

### Actores en Rust

#### Framework Actix

- Usa `tokio` y *futures* como runtime de sustento. Se ejecutan dentro del Sistema de Actores.
- El núcleo es de tipo `Arbitrer`: un thread que **crea un event loop** por debajo y **provee un handler**.
- Cada actor se ejecuta dentro de un Arbitrer.
- El handler se usa **para enviar mensajes al actor**.
- Se ejecutan en un contexto de ejecución `Context<A>`

#### Crear un actor

- Crear un tipo de dato. Debe implementar el trait `Actor`.
- Definir un mensaje e implementar el handler para este tipo del actor (`Handler<M>`). 
- Los mensajes pueden ser manejados de forma asincrónica.
- Crear el actor y hacer *spawn* en uno de los árbitros.

#### Ciclo de vida de un actor

- **Iniciado (Started)**: con el método `started()` (implementación default del trait `Actor`, no lo implementamos). El contexto del actor pasa a estar creado y disponible.
- **En ejecución (Running)**: estado siguiente a la ejecución de `started()`. Puede estar allí **de forma indefinida**.
- **Parando (Stopping)**: puede pasar a ese estado si:
	- Se llama a `Context::stop` en el mismo actor.
	- Ningún otro actor lo referencia (esto lo detecta el Framework y lo detiene).
	- No hay objetos registrados en el contexto.
- **Detenido (Stopped)**: desde el estado anterior no modificó su situación, por lo tanto continúa y **se detiene**. Es el último estado de ejecución.

#### Dirección

```rust
struct MyActor;

impl Actor for MyActor {
	type Context = Context<Self>;
}

let addr = MyActor.start();
``` 

Los actores son referenciados únicamente por la dirección.

- Al ejecutar `start()` en Main, **crea al actor** y devuelve **la dirección de ese actor**. Luego se la puede pasar a otros actores para que le manden mensajes.

#### Mensaje

- Un actor se comunica con otro **enviando mensajes**.
- Todos los mensajes son **tipados**, deben implementar el trait `Message`.
- `Message::Result` define el tipo de retorno.

```rust
impl Message for Ping {
	type Result = Result<bool, std::io::Error>;
	// Se quiere que Ping devuelva un Boolean o un error de la 'std'
}
```

Indica que ese Struct **puede utilizarse como un mensaje**.

#### Enviar mensaje

Existen varias formas para enviar un mensaje hacia otro actor:

- `Adrr::do_send(M)`: **ignora errores en el envío** del mensaje `M`. Si la casilla está cerrada, se descarta. No retorna el resultado.
- `Adrr::try_send(M)`: **trata de enviar el mensaje** inmediatamente. Si la casilla está llena o cerrada, retorna `SendError`.
- `Addr::send(M)`: retorna el objeto *future* que es el **resultado del proceso de manejo del mensaje** por el otro actor.

#### Contexto

- Los actores mantienen el **contexto interno de ejecución**, o **estado**.
- El Struct de contexto permite al actor:
	- Determinar su dirección
	- Cambiar los límites de la casilla de mensajes
	- Llamar al `stop` o detenerse
- Los mensajes llegan a la casilla primero, luego el contexto de ejecución **llama al handler específico**.

#### Arbitrer

- **Provee el contexto de ejecución** asincrónica para los actores, funciones y *futures*.
- **Aloja el entorno** donde se ejecuta el actor.
- Realizan varias tareas:
	- Crear un nuevo thread del SO
	- Ejecutar un event loop
	- Crear tareas asincrónicas en ese event loop