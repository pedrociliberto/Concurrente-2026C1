## Clase 3 - Programación Asincrónica

### Tareas asincrónicas

Cuando empezamos a tener muchísimos *threads*, el costo total **aumenta**. La demanda de memoria puede ser un problema.

Se pueden usar **tareas asincrónicas de Rust** (y en otros lenguajes) para intercalar tareas en un solo thread o en un pool de threads.

- Estas tareas son **mucho más livianas que los threads**.
- Son más rápidas de crear, y es más eficiente pasarle el control a ellas.
- Hay menor overhead de memoria.
- Se puede tener miles en un programa.
- El código asincrónico se ve igual que el de threads, aunque **las operaciones bloqueantes se manejan diferente**.

Se le suele llamar **continuaciones**: tareas que se interrumpen para cederle tiempo de CPU a otras tareas. Cada una define un punto a partir del cual "dejar de hacerse", y continuarse luego.

A diferencia de los hilos, **la tarea asincrónica voluntariamente cede el control**, y es responsable de su propio contexto. Los hilos pueden ser interrumpidos externamente (por el sistema operativo). 

Los puntos en donde la tarea cede el control, serán **puntos de espera** (especialmente de entrada/salida).

### Future

Se define en Rust el trait `Future`.

- Es un modelo de "piñata": a un elemento `Future` se le puede hacer `.poll()` ("pinchar"), y devolver un `Poll`, determinando si está **listo** (*Ready*) o **pendiente** (*Pending*). Se lo "golpea con el `.poll()`" hasta obtener el valor.
- Cuando ya está listo, devolverá `Ready` con cualquier resultado posible, dependiendo de si terminó correctamente o con algún error.
- Si devuelve `Pending` significa que está esperando algo y no terminó.
- El método `.poll()` **no bloquea**.
- Cada vez que es polleado, avanza todo lo que puede. Almacena lo necesario para realizar el pedido hecho por la invocación.
- El crate `async-std` provee versiones de las facilidades de I/O de la *std*.

**Performance**:
- La arquitectura *async* de Rust está hecha para ser eficiente.
- Se llama a `poll` únicamente cuando vale la pena (algo debe retornar `Ready`, o progresar al objetivo).

### Funciones *async*

- Invocar una función *async* **retorna inmediatamente**, antes que comienze a ejecutarse el cuerpo.
- Se obtiene un `Future` del valor: tiene los argumentos y espacio para variables para que la función pueda ejecutarse.
- El tipo específico se genera al compilar.
- Al ejecutar `poll` por primera vez sobre el retorno, se ejecuta el cuerpo de la función **hasta el primer await**.
- Si no se completó, retorna `Pending` y toda la función devuelve ese valor/estado.
- La expresión *await* toma ownership del future y hace el `poll`.
- Si está `Ready`, el valor final del Future es el devuelto en la expresión *await*.

Entonces se puede ver de esta manera:
- `async fn`: devuelve un **Future**. Llamarla **no ejecuta nada**.
- `.await`: ejecuta (puede pausar).
- El código se ejecuta en partes, no dodo junto.
- `poll`: hace que avance el Future, corre hasta el próximo `.await`. Devuelve `Ready` si terminó, o `Pending` si falta. Guarda el estado para continuar después.
- Los `.await` solo pueden usarse dentro de funciones *async*.

### Cuándo usar *async*

- Consultar servicios externos
- Leer de un archivo
- Server requests HTTP

### Block on

Se usa `block_on` para que el hilo de ejecución actual **espere por una tarea**. Se conecta el mundo sincrónico con el asincrónico. 

### Join

Dada una tarea, poder hacerle `.await` a múltiples tareas concurrentemente.

### Pin

Tipos que **no pueden moverse de lugar** una vez alocados en la memoria.

- Todos los tipos por defecto implementan el *autotrait* `Unpin`, **a excepción de los `!Unpin`**.
- Las self-references se encierran en el tipo `Pin` (ejemplo: `Pin<Box<T>>`)
- `Pin` evita que se mueva haciendo imposible llamar métodos que requieran `&mut T` como `mem::swap`.

### Runtimes

Rust no trae nada para **correr cosas asincrónicas**. Solo trae soporte de compilador para traducir *async await* con Futures.

Existen bibliotecas para suplir las demás necesidades asincrónicas.

- Tokio
- Async-std

Estos *runtimes* incluyen:
- **Exectutors**: cada cuánto se hace `poll`. Con qué frecuencia y sobre qué thread(s).
- **Bibliotecas**: async IO, timers, locks, channels, etc.
- **Utilidades**: tokio console.