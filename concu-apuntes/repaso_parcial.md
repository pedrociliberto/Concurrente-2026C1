# Apunte de Repaso - Parcial 06/05/2026

## Modelos de concurrencia

Durante las clases hemos visto varios modelos de concurrencia, y cada uno es distinto por su forma de manejar los procesos concurrentes. Con sus ventajas, están destinados a servir más para determinadas tareas por encima de otras.

### 1. Fork-Join

- Dividir una tarea grande en subtareas pequeñas (**Fork**).
- Ejecutarlas en paralelo.
- Esperar a que todas terminen para combinar sus resultados (**Join**).

#### Herramientas usadas

- `rayon`, `rayon::scope`, `par_iter`
- `thread::spawn`

#### Para qué sirve

- Ideal para **tareas intensivas de CPU** que pueden dividirse fácilmente.
- **Procesamiento de imágenes/video** (como el ejemplo visto en clase).
- **Motores de búsqueda**: dividir la búsqueda de una palabra en diferentes índices de datos.
- **Algoritmos de ordenamiento**: MergeSort, QuickSort, etc.
- Aprovecha al máximo todos los núcleos del procesador.

### 2. Programación Asincrónica (`async`)

- No bloquear al hilo de ejecución mientras se **espera una respuesta externa (I/O)**.
- El programa "suspende" una tarea y se dedica a otra hasta que la respuesta llega.

#### Herramientas usadas

- `async_std` o `tokio`
- `async`, `await`
- `Future`
- `task::block_on`
#### Para qué sirve

- Cuando el programa se pasa mucho tiempo esperando (I/O), no calculando.
- Permite manejar muchísimas conexiones simultáneas con pocos recursos de memoria (hilos).
- **Servidores web**: atender a muchos clientes que piden datos a una BDD.
- **Interfaces de usuario (UI)**: para que la ventana no se "congele" mientras se descarga un archivo de internet.

### 3. Estado mutable compartido

- Varios hilos acceden y modifican **la misma posición de memoria**.
- Requiere mecanismos de sincronización como **Mutex** o **Semáforos** para evitar que choquen al escribir.

#### Herramientas usadas

- `Arc<T>`: varios hilos son dueños del mismo dato
- `Mutex<T>` (exclusión mutua)
- `RwLock<T>` (lectura/escritura)
- `std_sempahore::semaphore`: semáforos
- `std::sync::Condvar`: variables de condición
#### Para qué sirve

- Cuando la velocidad de acceso a los datos es crítica y los datos **deben estar en un solo lugar**.
- Modelo de más bajo nivel y mayor rendimiento si se gestiona bien.
- **Motores de videojuegos**: física y gráficos leyendo posiciones de jugadores constantemente.
- **Sistemas operativos**: gestionar acceso a recursos de hardware.

### 4. Pasaje de mensajes / Actores

- Los hilos **no** comparten memoria.
- Solo se pueden enviar mensajes a través de un canal o "buzón". 
- Cada actor es dueño exclusivo de su estado.
#### Herramientas usadas

- `std::sync::mpsc`: **canales**, múltiples productores, un solo consumidor
- `async_std::channel` o `tokio::sync::mpsc`: canales asincrónicos
- `Actix`: framework de Actores
#### Para qué sirve

- Para sistemas complejos, distribuidos o que necesitan ser tolerantes a fallos.
- Elimina problemas de *Race Conditions* y no hay memoria compartida.
- **Sistemas de Telecomunicaciones**, como WhatsApp
- **Sistemas bancarios**: generar consistencia en transacciones, cada cuenta puede verse como un actor independiente.

### Tabla comparativa

| **Modelo**                    | **Ventajas Principales**                                                                        | **Herramientas en Rust**                                              | **Aplicaciones en la Vida Real**                                                 |
| ----------------------------- | ----------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| **Vectorización (SIMD)**      | Máximo **rendimiento por núcleo CPU**. Ejecuta una instrucción sobre múltiples datos.           | Auto-vectorización del compilador, Intrínsecos de CPU (`std::arch`).  | Procesamiento de señales, criptografía, física de videojuegos y álgebra lineal.  |
| **Fork-Join**                 | Paralelismo estructurado. **Divide tareas pesadas** de CPU de forma automática.                 | **Rayon** (`par_iter`, `scope`), `std::thread`.                       | Renderizado de imágenes, compresión de archivos y Big Data (MapReduce).          |
| **Asincrónico (I/O)**         | Alta escalabilidad con pocos recursos. No bloquea el hilo mientras espera (**espera por I/O**). | `async`/`await`, **Tokio**, **async-std**, `futures` crate.           | Servidores web (APIs), microservidores, aplicaciones de chat y proxies.          |
| **Estado Mutable Compartido** | **Acceso directo a memoria**. Menor latencia de comunicación entre hilos.                       | `Arc<T>`, `Mutex<T>`, `RwLock<T>`, `Condvar`, `Semaphore`, `Barrier`. | Motores de bases de datos, núcleos de sistemas operativos y software de trading. |
| **Mensajería (Actores)**      | Aislamiento total. Evita errores de memoria y facilita **sistemas distribuidos**.               | `std::sync::mpsc`, canales de Tokio/Crossbeam, **Actix**.             | Sistemas bancarios, sistemas de telecomunicaciones (WhatsApp) y microservicios.  |

## Conceptos clave

### Busy Wait

El **Busy Wait** ocurre cuando un hilo **se mantiene en un bucle infinito** verificando constantemente si una condición se cumple (por ejemplo, si un `Mutex` se liberó o si llegó un mensaje), en lugar de suspenderse y ceder el procesador.

- **El problema:** El hilo consume el **100% de un núcleo de CPU** sin realizar trabajo útil, simplemente preguntando "¿Ya está?". Esto quita recursos a otros hilos que sí tienen tareas pendientes.
- **La alternativa:** El **bloqueo pasivo**, donde el sistema operativo pone el hilo a "dormir" y lo despierta solo cuando la condición cambia (usando `Condvar` o señales del OS).

#### ¿Cómo se ocasiona en Rust?

##### 1. Bucles sobre variables atómicas o compartidas

Es la forma más común. Un hilo se queda en un `while` esperando que otro cambie un valor.

```rust
// MAL: Esto es Busy Wait
while !READY.load(Ordering::SeqCst) {
    // La CPU vuela al 100% aquí dentro sin hacer nada
}
```

##### 2. Uso incorrecto de `try_lock()`

Si intentas obtener un bloqueo de forma no bloqueante dentro de un bucle sin pausas.

```rust
// MAL: Reintento constante sin descanso
loop {
    if let Ok(guard) = my_mutex.try_lock() {
        break;
    }
    // No hay pausa, la CPU se satura reintentando el lock
}
```

#### Cómo evitarlo (Buenas Prácticas)

Para que tu apunte esté completo, añade estas soluciones:

- **Usar `Condvar`:** En lugar de un bucle, usa `condvar.wait(guard)`. Esto libera el mutex y duerme el hilo.
- **Usar Canales:** Un `receiver.recv()` bloquea el hilo automáticamente hasta que llega un mensaje.
- **En Async:** Nunca uses bucles bloqueantes. Usa `.await` o `tokio::task::yield_now()` si necesitas ceder el turno.
### `Poll` y funciones `async`

#### 1. ¿Qué es una función `async`?

Cuando definimos `async fn`, el compilador transforma tu código en una **máquina de estados** que implementa el rasgo `Future`.

- Cada vez que escribimos `.await`, definimos un "punto de suspensión". 
- La función no se ejecuta hasta el final de un tirón; se ejecuta por fragmentos entre cada `.await`.
#### 2. El concepto de `Poll` (La encuesta)

El corazón de un `Future` es el método `poll`. Un _Runtime_ (como Tokio) no "sabe" cuándo un futuro está listo; tiene que preguntarle.

- **`Poll::Pending`**: El futuro le dice al runtime: "Todavía no terminé, estoy esperando algo (un timer, un socket)". El hilo queda libre para hacer otra cosa.
- **`Poll::Ready(valor)`**: El futuro dice: "Listo! Acá tenés el resultado".

### Espacios de memoria

Quise detallar cómo funciona el espacio de memoria de todas las entidades vistas:
#### 1. Procesos

Un proceso es la unidad de aislamiento más fuerte del Sistema Operativo.

- **Memoria:** Cada proceso tiene su propio espacio de direccionamiento virtual. Tiene su propio **Stack** y su propio **Heap**.
- **Aislamiento:** Un proceso **no puede** leer ni escribir en la memoria de otro proceso a menos que usen mecanismos especiales (IPC).
- **Costo:** Muy alto (cambio de contexto pesado).
#### 2. Hilos (Threads / Fork-Join)

Los hilos viven **dentro** de un proceso.

- **Memoria Compartida:** Todos los hilos de un mismo proceso comparten el mismo **Heap**, las variables globales y el código.
- **Memoria Privada:** Cada hilo tiene su propio **Stack** privado (generalmente de 2MB por defecto en Linux) para sus variables locales y llamadas a funciones.
- **Aspecto Crítico:** Al compartir el Heap, es donde surgen las condiciones de carrera y necesitamos los `Mutex`.
#### 3. Tareas Asincrónicas (Futures)

Son mucho más livianas que los hilos y son gestionadas por el lenguaje/runtime, no por el SO.

- **Memoria:** No tienen un Stack propio del sistema operativo. Se guardan como **estructuras de datos en el Heap** (o el stack del hilo que las ejecuta en ese momento).
- **Comportamiento:** Cuando una tarea asincrónica hace `.await`, su "estado" (las variables locales que necesita conservar) se empaqueta en una máquina de estados y se queda en el **Heap** hasta que el runtime la vuelve a llamar.
- **Costo:** Mínimo (se puede tener millones de tareas asincrónicas con la memoria que usarían unos pocos cientos de hilos).
#### 4. Actores

El modelo de actores es una abstracción sobre hilos o tareas asincrónicas.

- **Memoria:** Cada actor tiene su propio **estado interno privado**. Aunque físicamente vivan en el mismo Heap que otros actores (dentro del mismo proceso), la regla de diseño es que **nadie más puede tocar esa memoria**.
- **Comunicación:** La única forma de "ver" o "cambiar" algo en un actor es **enviándole un mensaje** que se copia o se mueve a su buzón de entrada.

#### Comparación de Estructura de Memoria

| **Entidad**       | **¿Tiene Stack propio?**        | **¿Comparte Heap?**            | **Gestión por...**   | **Unidad de aislamiento** |
| ----------------- | ------------------------------- | ------------------------------ | -------------------- | ------------------------- |
| **Proceso**       | Sí (Privado)                    | No (Aislado)                   | Sistema Operativo    | Hardware / MMU            |
| **Hilo (Thread)** | Sí (Privado)                    | **Sí** (Compartido)            | Sistema Operativo    | Pila de llamadas          |
| **Tarea Async**   | No (Usa el del ejecutor)        | **Sí** (Estado en Heap)        | Runtime (Rust/Tokio) | Máquina de estados        |
| **Actor**         | N/A (Depende de implementación) | Sí (Pero prohibido por diseño) | Framework (Actix)    | Mensajería                |
#### Análisis de "lo que comparten"

1. **Hilos vs Procesos:** Los hilos comparten casi todo excepto el Stack y los registros de la CPU. Esto los hace rápidos para comunicarse pero peligrosos para la seguridad de datos (usar exclusión mutua).
2. **Async vs Hilos:** Las tareas asincrónicas son "recurrentes" sobre los hilos. Un hilo de Rayon o Tokio puede ejecutar miles de tareas asincrónicas. Lo que comparten es el hilo físico.
### Operaciones en Vectorización

#### Diferencias Clave

| **Característica** | **Operación Vertical**                                     | **Operación Horizontal**                                                   |
| ------------------ | ---------------------------------------------------------- | -------------------------------------------------------------------------- |
| **Entrada**        | Dos o más registros.                                       | Un solo registro (usualmente).                                             |
| **Resultado**      | Un **nuevo vector** (varios valores).                      | Un **escalar** o un **vector** reducido (un valor).                        |
| **Uso común**      | Sumar dos arreglos, aplicar filtros, multiplicar matrices. | Calcular el promedio de un vector, hallar el valor máximo o la suma total. |
| **Rendimiento**    | Muy alto (paralelismo real).                               | Moderado (implica dependencias internas).                                  |
### Condvars

#### 1. ¿Para qué se usan las Condvars?

Su uso principal es evitar el **Busy Wait**. En lugar de tener un hilo en un bucle consumiendo CPU para ver si una cola tiene datos o si un recurso está libre, **el hilo se suspende**.

- **El Mutex protege el dato.**
- **La Condvar protege la espera.**
#### 2. El método `wait_while` en Rust

En Rust, las condvars se usan siempre junto a un `Mutex`. El método `wait_while` es una utilidad de conveniencia que combina un bucle de verificación con la suspensión del hilo.

**¿Cómo funciona internamente?**

1. Bloqueás el `Mutex`.
2. Llamas a `wait_while(guard, condicion)`.
3. Si la condición es falsa, el hilo libera automáticamente el `Mutex` y se duerme.
4. Cuando otro hilo llama a `notify_one()` o `notify_all()`, tu hilo se despierta, **vuelve a bloquear el Mutex** y verifica la condición de nuevo.

```rust
use std::sync::{Arc, Mutex, Condvar};
use std::thread;

let pair = Arc::new((Mutex::new(false), Condvar::new()));
let pair2 = Arc::clone(&pair);

thread::spawn(move || {
    let (lock, cvar) = &*pair2;
    let mut started = lock.lock().unwrap();
    *started = true; // Cambiamos el estado
    cvar.notify_one(); // Avisamos al que espera
});

let (lock, cvar) = &*pair;

// wait_while se encarga de re-chequear si 'started' es true
cvar.wait_while(lock.lock().unwrap(), |started| !*started).unwrap();
``` 

#### 3. Spurious Wakeups (Despertares Espurios)

Un **spurious wakeup** es un fenómeno donde un hilo se despierta de su estado de espera en la Condvar **sin que nadie haya llamado a `notify`**, o sin que la condición realmente se haya cumplido.

- **¿Por qué ocurren?** Suceden por la forma en que los sistemas operativos (POSIX o Windows) gestionan las colas de hilos y las interrupciones a bajo nivel. Optimizar el hardware para garantizar que _nunca_ haya un despertar falso sería demasiado costoso en rendimiento.
- **La consecuencia:** Nunca debes asumir que si el hilo se despertó, la condición es verdadera.
- **La solución en Rust:** Por eso se usa un bucle (como hace `wait_while`). El hilo se despierta, mira el dato, y si la condición para esperar sigue siendo verdadera, se vuelve a dormir inmediatamente.