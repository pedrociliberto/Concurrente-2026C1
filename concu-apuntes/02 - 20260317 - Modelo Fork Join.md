## Clase 2 - Modelo Fork Join

### Nociones principales

**Fork-join**: paralelización donde el cómputo (*task*) es **dividido en sub-cómputos menores** (*subtasks*). Los resultados se unen (*join*) para construir la solución al cómputo inical.

Como condición para resolverlo de esta manera, el cómputo **debe poderse dividir en subtareas**. 

- Partir el cómputo se suele realizar **de forma recursiva**. Los subcómputos son independientes, por lo que todo puede realizarse **en paralelo**.
- Las subtareas se pueden crear en cualquier momento de la ejecución de la tarea principal. Es decir, el cómputo inicial puede dividirse en cualquier momento del proceso.
- Las tareas **no deben bloquearse**, excepto para esperar el final de las subtareas.

**Fork-Join** es un modelo de concurrencia ***sin condiciones de carrera***.

- Los programas *fork-join* son **determinísticos**, y los threads están aislados. El programa arroja el mismo resultado sin importar que algunos threads sean más veloces que otros.
- Idealmente, se desea una *performance* de `t_secuencial / N_threads` (tiempo total si todo se hiciera secuencial, dividido la cantidad de threads usados). Puede variar por el tamaño de las tareas, y porque hay que **combinar los resultados invididuales**.
- Desventaja: **las tareas deben ser aisladas** entre sí.

Se crean tantos threads como núcleos (CPUs) tenga la computadora.
### Work Stealing

*¿Cómo se implementa de forma eficiente este modelo de concurrencia?*

***Work Stealing*** es un algoritmo usado para hacer scheduling de tareas entre threads. Los *worker threads* inactivos **roban trabajo** a threas ocupados, para balancear la carga de trabajo.

- Cada thread tiene su propia **cola de dos extremos** (*deque*), para almacenar las tareas que ya se pueden ejecutar.
- Cuando un thread termina una tarea, coloca las **subtareas creadas al final de la cola**.
- Luego toma la siguiente tarea a ejecutar **del final de la cola**.
- El thread no tiene más trabajo **cuando se vacía la cola**: intenta *robar* tareas del inicio de una cola de otro thread (elección al azar).

**Ventajas**:
- Los *worker threads* se comunican **solo cuando lo necesitan**. Hay menor necesidad de sincronización.
- La implementación de la cola *deque* agrega bajo overhead de sincronización.

### Rayon

Biblioteca muy popular de Rust. Implementa el modelo *fork-join* de 2 formas.
#### Realizar dos tareas en paralelo

```rust
let (v1, v2) = rayon::join(fn1, fn2);

// Invoca a fn1 y fn2, retornando una tupla con ambos resultados. El tiempo de procesamiento debería ser similar al de la función que más tarda.
```

#### Realizar N tareas en paralelo

```rust
giant_vector.par_iter().for_each(|value| {
	do_thing_with_value(value);
});

// Se crea un iterador 'ParallelIterator' similar a los iteradores de Rust. Rayon maneja los threads y distribuye el trabajo con Work Stealing.
```

**Rayon** parece crear una tarea por elemento del vector, pero internamente, **crea un *worker thread* por núcleo del CPU**.

Los métodos `.reduce()` y `.reduce_with()` se usan en Rayon para combinar resultados.

```rust
use rayon::prelude::*; 

let s = ['a', 'b', 'c', 'd', 'e']
	.par_iter()
	.map(|c: &char| format!("{}", c))
	.reduce(|| String::new(),
		|mut a: String, b: String| 
		{ a.push_str(&b); a }); 

assert_eq!(s, "abcde");
```

### Crossbeam

Crate de concurrencia que **estructuras de datos y funciones** para concurrencia y paralelismo. `crossbeam::scope` crea un nuevo entorno de thread en donde:

- Se garantiza que todos los threads terminan **antes de retornar el closure** que se le pasa como argumento a esta función.
- Todos los threads que no fueron manualmente esperados (*join*) son esperados antes que finalice la invocación de la función.
- Se devuelve `Ok` si todos terminan exitosamente, y `Err` si al menos alguno ejecutó `panic`.

