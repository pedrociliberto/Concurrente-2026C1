## Clase 4 - Corrección / Sección Crítica / Locks

### Corrección

- En procesos (programas secuenciales) podemos **debuggear para encontrar errores**. Ante una entrada se obtiene siempre una misma salida.
- En programas concurrentes, la salida puede **depender del escenario de ejecución**.
#### Propiedades

**De tipo Safety**: deben ser verdaderas siempre.
- **Exclusión mutua**: 2 procesos no deben intercalar instrucciones. 
- **Ausencia de *deadlock***: un sistema andando debe poder continuar realizando su tarea (avanzar productivamente).

**De tipo Liveness**: deben ser verdaderas eventualmente.
- **Ausencia de *starvation***: todo proceso listo para usar un recurso debe recibirlo eventualmente.
- ***Fairness***: un escenario es *fair* si en algún estado, una instrucción continuamente habilitada aparece en algún momento en el escenario.

### Sección Crítica

Cada proceso se ejecuta en un *loop* infinito posiblemente dividido en secciones **críticas** y **no críticas**.

**Especificaciones de corrección**: 
- **Exclusión mutua**: no deben intercalarse instrucciones en la sección crítica.
- **Ausencia de *deadlock***: si 2 procesos quieren entrar, eventualmente alguno de ellos debe tener éxito. Si ninguno pudiera entrar, hay *deadlock*.
- **Ausencia de *starvation***: si un proceso quiere entrar, eventualmente debe poder entrar.

Entonces:
- La sección crítica debe progresar (finalizar eventualmente).
- La sección no crítica no requiere progreso (puede terminar o entrar en un *loop* infinito).

### Locks

- Sirven para realizar **exclusión mutua** entre procesos.
- Se implementan con variables de tipo `lock`, que contienen su estado.
- Se usan los métodos `lock()` y `unlock()`:
	- `lock()`: el proceso se bloquea hasta poder obtener el lock.
	- `unlock()`: el proceso libera el lock que tomó previamente con `lock`.
- Se necesita soporte tanto del hardware como del sistema operativo.

### Locks en UNIX

- Mecanismo de sincronismo de acceso a un archivo (se pueden usar para cualquier otro recurso).
- Los locks serán manipulaciones del *file descriptor*.
- En Unix son *advisory*: los procesos pueden ignorarlos.
#### Condiciones

Para poder tomar un:
- *Shared lock (read)*: el proceso debe esperar hasta que se liberen todos los *exclusive locks*. Más de un proceso puede tenerlo a la vez.
- *Exclusive lock (write)*: el proceso debe esperar hasta que se liberen **todos los locks** (de ambos tipos). Solo uno puede tenerlo a la vez.

### Locks en Rust

#### Trait `Send`

- Indica que el ownership del tipo que lo implementa puede ser transferido entre *threads*.
- Casi todos los tipos de Rust son `Send` (`Rc<T>` no lo es).
- Los tipos compuestos formados por tipos `Send` son automáticamente del mismo trait. 

#### Trait `Sync`

- Indica que es seguro (para el que lo implementa) ser referenciado desde múltiples *threads*.
- T es `Sync` si `&T` es `Send`.
- Los tipos primitivos son `Sync`, y los compuestos formados por tipos `Sync` también lo son.

#### Uso de locks

Rust provee locks **compartidos** (de lectura) y **exclusivos** (de escritura) en el módulo `std::sync::RwLock`.

- No hay una política específica; es dependiente del sistema operativo.
- Se requiera que `T` sea `Send` para ser compartido ente threads, y `Sync` para permitir acceso concurrente ente lectores.

```rust
use std::sync::RwLock;

let lock = RwLock::new(5);
```

##### Obtener locks

```rust
// Obtener lock de lectura
fn read(&self) -> LockResult<RwLockReadGuard<T>>
// Bloquea al thread hasta poder obtenerlo con acceso compartido. Puede haber otros con este lock.


// Obtener lock de escritura 
fn write(&self) -> LockResult<RwLockReadGuard<T>>
// Bloquea al thread hasta poder obtenerlo de forma exclusiva. Solo él puede tenerlo.
```

- Ambas retornan una **protección** (guarda) que libera el lock con RAII.
- Una vez obtenido, se puede acceder al valor protegido.

##### Locks Envenenados

- Un lock queda **envenenado** cuando un thread lo toma de forma **exclusiva** y ejecuta `panic!` teniendo el lock.
- El lock se libera luego de que esto ocurre, pero queda envenenado.
- Las llamadas posteriores a `read()` y `write()` sobre ese lock devolverán `Error`.