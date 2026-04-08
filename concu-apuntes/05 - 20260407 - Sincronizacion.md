## Clase 5 - Sincronización

### Diagrama de proceso básico

- Se ejecuta una *system call* que **crea un proceso**.
- El sistema operativo lo tiene que **admitir** (verificar y alocar los recursos para que el proceso pueda ejecutarse).
- Una vez admitido, el proceso pasa a  **estado listo** (preparados para que el *scheduler* los seleccione).
- Cuando lo selecciona, el proceso está **en estado de ejecución**, y usa tiempo de CPU.
- Cuando un hilo espera por la ocurrencia de un evento, está en **estado bloqueado**. Su ejecución está en espera.
- Con la *system call* `exit`, el proceso pasa a estar **finalizado**. Queda en estado **zombie**.

### Semáforos

- Mecanismos de sincronismo.
- Implementado como una construcción de programación concurrente **de más alto nivel**.
- Tipo de dato compuesto por dos campos:
	- Entero no negativo: `V`
	- Set de procesos: `L`
- Se inicializa con un valor `k >= 0` y con el conjunto vacío.
- Hay dos operaciones atómicas sobre un semáforo `S`:
	- `wait(S)`: también llamada `p(S)`
	- `signal(S)`: también llamada `v(S)`

 Son mecanismos de sincronismo de **acceso a un recurso**. El semáforo **es un contador**:
 - Si `contador > 0`: recurso disponible
 - Si `contador <= 0`: recurso disponible
- El valor de semáforo representa la **cantidad de recursos disponibles**.
- El set de procesos indica los **procesos en espera** por conseguir recursos.
- Si el valor es 0 o 1, son **semáforos binarios** y se comportan igual que los **locks de escritura** (`Mutex`)

Operaciones:
- **p** (`wait`): resta 1 al contador
- **v** (`signal`): suma 1 al contador

#### Operaciones
##### *Wait* (p)

Cuando un proceso `p` quiere utilizar un recurso:
- Si hay recursos disponibles, toma uno restando 1 al contador, y continúa su ejecución.
- Si no hay recursos disponibles, `p` se añade a la lista de espera y se bloquea.

```
if S.V > 0
	S.V := S.V - 1
else:
	S.L add p
	p.state := blocked
```

##### *Signal* (v)

Cuando un proceso termina de usar un recurso y lo libera:
- Si no hay nadie esperando en la lista, solo se incrementa el contador de recursos disponibles.
- Si hay procesos esperando, se elige arbitrariamente un proceso `q` de la lista. Se lo quita de la lista de espera, y **se lo pone como "listo**", para que el SO lo ejecute para que use el recurso liberado.

```
if S.K is empty
	S.V := S.V + 1
else:
	q := <elemento arbitrario de S.L>
	S.L remove q
	q.state := ready
```

#### Semáforo binario o Mutex

- `V` solo puede valer 0 o 1.
- Se inicializa como (0, ∅) o (1, ∅).
##### *Signal* (v)

```
if S.V = 1
	// UNDEFINED (no puede ocurrir)
else if S.L is empty
	S.V := 1
else:
	q := <elemento arbitrario de S.L>
	S.L remove q
	q.state := ready
```

#### Resumen

**Recordar**:
- `wait()` y `signal()` son instrucciones **atómicas**.
- Debe ser inicializado con un **valor entero no negativo**.
- `signal()` debe despertar a uno de los procesos suspendidos, pero no está definido cúal.

**Invariantes**:
- `S.V >= 0`
- `S.V = k + #signal(S) - #wait(S)`, siendo `k` el valor inicial del semáforo.

### Implementación de Semáforos

Tipos de semáforos: 
- System V
- POSIX

#### System V

Un semáforo **System V** se compone por:
- El valor del semáforo;
- El *process id* del último proceso que lo utilizó;
- La cantidad de procesos esperando por el semáforo;
- La cantidad de procesos esperando que el semáforo sea cero (0).

#### Semáforos en Rust

Usamos el crate **`std-sempahore`**.

- Inicializar el semáforo: `let sem = Semaphore::new(5);`. `5` es el valor inicial del semáforo.
- Obtener el acceso (wait): `fn acquire(&self)`
- Liberar el semáforo (signal): `fn release(&self)`
- *(extra)* Obtener el acceso con el patrón **RAII** (wait): `access(&self)`. Una vez obtenida la guarda, se activa automáticamente el *release* (no hace falta liberarlo).

### Barreras en Rust

Permiten **sincronizar varios threads** en puntos determinados de un cálculo/algoritmo. Una vez llega el último (a sincronizarse), se abre la barrera para dejarlos libres para seguir.

Están en el módulo `std::sync::Barrier`:
- **Creación de barrera**: `fn new(n: usize) -> Barrier`
- **Bloquear al thread** hasta que todos lleguen al punto: `fn wait(&self) -> BarrierWaitResult`

El método `BarrierWaitResult::is_leader()` devuelve **True** en el thread líder. Son útiles si necesitamos **comportamiento distinto a los demás** en un solo thread.

Las barreras son **reutilizables automáticamente**.

### Problemas clásicos: Productor - Consumidor

- Se definen 2 familias de procesos: **productores y consumidores**.
- **Requisitos** (premisas/propiedades/invariantes):
	- **No** se puede **consumir lo que no hay**.
	- Todos los items producidos son **eventualmente consumidos**.
	- Se **accede de a uno** al espacio de almacenamiento.
	- Se debe respetar el orden de **almacenamiento y retiro** de los items.
- Al usar un buffer de comunicación existen estos **problemas de sincronización**:
	1. No se puede consumir si el buffer está vacío.
	2. No se puede producir si el buffer está lleno.

Existen 2 casos: 

- **Buffer infinito**: solo está el 1er problema
- **Buffer acotado**: se presentan ambos problemas

#### Buffer infinito

- Se crea un buffer como una cola vacía.
- El semáforo **modeliza cuánto recurso hay en el buffer**. 

```
buffer := emptyQueue
sem notEmpty (0, ∅)
```

##### Productor

- Define un **tipo de dato** para el producto.
- **Agrega el producto** al buffer. Lo puede hacer siempre porque el buffer es infinito.
- Hace *signal* al semáforo `notEmpty`, sumándole 1 al contador (es decir, hay un recurso disponible).

```
datatype d
loop forever
p1: append(d, buffer)
p2: signal(notEmpty)
```

##### Consumidor

- El consumidor **se bloquea** esperando a que haya recursos para consumir. Como el semáforo se inicializa en `0`, va a esperar inicialmente a que se produzca un recurso.
- Cuando aparece, **extrae un elemento** del buffer.

```
datatype d
loop forever
q1: wait(notEmpty)
q2: d <- take(buffer)
```

#### Buffer acotado

- Inicialmente se tienen `N` huecos de recursos para producir (nuevo semáforo).

```
buffer := emptyQueue
sem notEmpty (0, ∅) 
sem notFull (N, ∅)
```
##### Productor

- Hace *wait* sobre el recurso que necesita: debe **haber espacio** para producir (semáforo `notFull`). Solo se bloquea si no hay espacio disponible.
- Si hay, lo produce y lo **agrega al buffer**.
- Hace *signal* ya que hay un **nuevo elemento** para consumir.

```
datatype d
loop forever
p1: producir
p2: wait(notFull)
p3: append(d, buffer)
p2: signal(notEmpty)
```

##### Consumidor

- **Espera** con el semáforo a que haya un elemento en el buffer.
- **Toma el elemento** del buffer una vez encontrado.
- Hace *signal* ya que **se liberó un espacio** en el buffer para producir más.
- **Consume** el elemento.

```
datatype d
loop forever
q1: wait(notEmpty)
q2: d <- take(buffer)
q3: signal(notFull)
q4: consume(d)
```

### Monitores

- Otra herramienta de sincronización.
- Permite a los hilos **tener exlusión mutua**.
- Los hilos pueden esperar (*block*) porque una condición se vuelva falsa.

El monitor consta de:
- Nombre;
- Variables internas;
- Procedimientos del monitor: rutinas que acceden a variables internas;
- Una interfaz pública (los procesos acceden a variables internas);
- Inicialización de variables internas;
- Conjunto de **condiciones de variables** que incorporan sincronismo.

Los procesos pueden tomar distintos **estados**:
- Esperando para entrar al monitor;
- Ejecutando el monitor (solo uno a la vez);
- Bloqueado en FIFO de variable de condición;
- Recién liberado de la *wait condition*;
- Recién completó una opración `signalC`
#### Variable de condición

Una variable de condición `C`:
- No guarda ningún valor
- Tiene asociado un FIFO
- Consta de 3 operaciones atómicos:
	- `waitC(cond)`
	- `signalC(cond)`
	- `empty(cond)`

#### Comparativa con Semáforos

En el **semáforo**:
- `wait` puede o no bloquear
- `signal` siempre tiene efecto
- `signal` desbloquea un proceso arbitrario
- Un proceso desbloqueado con `signal` puede continuar la ejecución inmediatamente

En el **monitor**:
- `wait` siempre bloquea
- `signal` no tiene efecto si la cola está vacía
- `signal` desbloquea el proceso del tope de la cola (FIFO)
- Un proceso desbloqueado con `signalC` debe esperar que el proceso señalizador deje el monitor.