# Apunte de repaso - Final 15/07/2026

### Modelos de concurrencia y Sincronización

#### Explicar el modelo de concurrencia **Fork-join**. Explicar algoritmo de work stealing. Explicar y justificar por qué no es conveniente tener una única cola de tareas en dicho algoritmo.

El modelo de concurrencia **Fork-join** y su algoritmo asociado, **Work Stealing**, se basan en la división eficiente de tareas para aprovechar el paralelismo de los procesadores.

**El Modelo Fork-Join**

Este modelo consiste en una estrategia de paralelización donde un cómputo o tarea principal se **divide de forma recursiva en sub-cómputos menores** (etapa de **Fork**).

- **Independencia:** Como condición fundamental, las subtareas deben ser independientes entre sí y estar aisladas para evitar condiciones de carrera.
- **Ejecución:** Estas subtareas pueden ejecutarse en paralelo. Los hilos no deben bloquearse, excepto para esperar que sus subtareas finalicen.
- **Unificación:** Una vez completadas las subtareas, sus resultados se combinan para construir la solución al cómputo inicial (etapa de **Join**).
- **Determinismo:** Los programas bajo este modelo son determinísticos; arrojan el mismo resultado sin importar la velocidad relativa de los hilos.

**Algoritmo de Work Stealing**

Para implementar el modelo Fork-join de forma eficiente, se utiliza el algoritmo de **Work Stealing** (robo de trabajo) para realizar el _scheduling_ de las tareas entre los hilos. Su funcionamiento se describe de la siguiente manera:

1. **Colas privadas:** Se crean tantos hilos (_worker threads_) como núcleos tenga la CPU, y **cada hilo posee su propia cola de dos extremos** (_deque_) para almacenar sus tareas.
2. **Gestión local (LIFO):** Cuando un hilo genera nuevas subtareas, las coloca al **final** de su propia cola. Para seguir trabajando, el hilo toma la siguiente tarea también del **final** de su cola.
3. **Robo de trabajo (FIFO):** Si un hilo termina sus tareas y su cola se vacía, se convierte en un hilo inactivo. Para balancear la carga, este hilo intenta "robar" una tarea del **inicio** de la cola de otro hilo (elegido al azar).

**¿Por qué no es conveniente una única cola de tareas?**

No es recomendable utilizar una única cola global para todos los hilos por las siguientes razones extraídas de las fuentes:

- **Sincronización y Overhead:** Una cola única requeriría que todos los hilos compitieran por el mismo recurso cada vez que necesiten extraer o añadir una tarea. Esto generaría una alta necesidad de mecanismos de sincronización (locks) constantes, aumentando el _overhead_ del sistema. En cambio, con colas privadas, los hilos se comunican **solo cuando es estrictamente necesario** (durante el robo), reduciendo significativamente la sincronización.
- **Escalabilidad:** El uso de _deques_ individuales permite que la mayoría de las operaciones se realicen de forma local y privada, lo que escala mucho mejor al aumentar el número de núcleos.
- **Eficiencia del robo:** El hecho de que el hilo dueño use la cola como una pila (LIFO, por el final) y los "ladrones" extraigan del principio (FIFO) minimiza la interferencia entre el hilo que trabaja y el que intenta robar, permitiendo una implementación con bajo costo de coordinación.

#### Explicar **Programacion Asincrónica**, comparar con threads y dar ventajas y desventajas de cada uno. Dar ejemplo de uso de cada uno en el que convenga usarlo.

La **programación asincrónica** es un modelo de concurrencia donde el programa se compone de tareas sencillas que **cooperan entre sí** en el uso de los recursos y la CPU. En este modelo, el hilo de ejecución no se bloquea mientras espera una respuesta externa (I/O); en su lugar, la tarea **suspende su ejecución voluntariamente** en "puntos de espera" (puntos de `.await`) para ceder el control y permitir que otras tareas progresen.

**Comparación entre Programación Asincrónica y Threads**

A diferencia de los hilos (threads), que son gestionados por el sistema operativo, las tareas asincrónicas son gestionadas por el **lenguaje o un runtime** (como `Tokio` o `async-std`). Las principales diferencias técnicas radican en su estructura y manejo:

- **Gestión de interrupciones:** Los hilos pueden ser interrumpidos externamente por el _scheduler_ del sistema operativo, mientras que una tarea asincrónica es responsable de su propio contexto y debe **ceder el control de forma voluntaria**.
- **Uso de memoria:** Cada hilo posee su propio **stack privado** (generalmente de 2MB), lo que genera una alta demanda de memoria cuando hay muchos. Las tareas asincrónicas no tienen stack propio; sus variables y estado se empaquetan en una **máquina de estados que vive en el Heap**.
- **Costo de contexto:** El cambio de contexto entre procesos o hilos es pesado y costoso para el sistema operativo. El costo de crear y pasar el control entre tareas asincrónicas es **mínimo**, lo que permite tener millones de ellas en un solo programa.

**Ventajas y Desventajas**

|Característica|**Threads (Hilos)**|**Programación Asincrónica (Async)**|
|---|---|---|
|**Ventajas**|Permite paralelismo real en múltiples núcleos. Acceso directo a memoria con baja latencia. Ideal para **cómputo intensivo**.|Alta escalabilidad con pocos recursos. No desperdicia CPU durante esperas de I/O. Puede manejar miles de conexiones simultáneas.|
|**Desventajas**|Alto consumo de memoria por hilo. Riesgo de condiciones de carrera y necesidad de locks complejos.|Requiere un runtime externo. Las operaciones bloqueantes requieren un manejo especial para no "congelar" el hilo.|

**Ejemplos de uso conveniente**

- **Cuándo usar Threads (o Fork-Join):**
    - **Procesamiento de imágenes o video:** Tareas que requieren un uso intensivo de la CPU y pueden dividirse en subtareas independientes para aprovechar todos los núcleos del procesador.
    - **Motores de videojuegos:** Donde la física y los gráficos deben leer y escribir posiciones constantemente con la menor latencia posible.
    - **Algoritmos de ordenamiento masivo:** Como MergeSort o QuickSort, que se benefician de la división de tareas pesadas.
- **Cuándo usar Programación Asincrónica:**
    - **Servidores Web:** Para atender a miles de clientes simultáneos que pasan la mayor parte del tiempo esperando datos de una base de datos o servicios externos.
    - **Interfaces de Usuario (UI):** Para evitar que la ventana se "congele" mientras el programa descarga un archivo de internet o consulta una API en segundo plano.
    - **Aplicaciones de Chat o Proxies:** Sistemas que gestionan un alto movimiento de entrada/salida de datos con tiempos muertos de espera.

#### Explicar **modelo de actores** y dar sus componentes. Explicar qué es el arbiter. Explicar qué es el contexto.

El **modelo de actores** es una primitiva de concurrencia diseñada para que cada actor gestione su propio estado interno de forma aislada, comunicándose exclusivamente a través de mensajes para evitar el uso de memoria compartida.

a. El Modelo de Actores y sus componentes

En este modelo, los actores son entidades **livianas** (se pueden crear miles) que encapsulan comportamiento y estado. Sus componentes principales son:

- **Estado Privado:** Es el estado interno del actor, el cual es **inaccesible para otros actores** y solo puede modificarse mediante el procesamiento de mensajes.
- **Dirección (Address):** Es el identificador o referencia necesaria para que otros actores puedan enviarle mensajes. En sistemas distribuidos, esta dirección puede ser remota.
- **Casilla de correo (Mailbox):** Es una cola de tipo **FIFO** donde se almacenan los mensajes recibidos antes de ser procesados por el actor uno a la vez.
- **Mensajes:** Son estructuras simples e **inmutables** que representan los datos o instrucciones que se envían entre actores de forma asincrónica.

b. El Arbiter (Árbitro)

El **Arbiter** es el componente que proporciona el entorno físico y lógico para la ejecución de los actores. Sus funciones principales incluyen:

- **Alojamiento del entorno:** Es el hilo del sistema operativo que crea y mantiene un **event loop** (bucle de eventos) donde se ejecutan los actores, funciones y _futures_.
- **Gestión asincrónica:** Provee el contexto de ejecución asincrónica necesario para procesar las tareas dentro de dicho event loop.
- **Handlers:** Provee un manejador (_handler_) que se utiliza para enviar los mensajes al actor de forma efectiva.

c. El Contexto (Context)

El **Contexto** representa el entorno interno de ejecución de un actor específico y sirve como puente entre el actor y el framework que lo sustenta. Sus capacidades fundamentales son:

- **Gestión del Actor:** Permite al actor conocer su propia **dirección**, modificar los límites de su casilla de correo o solicitar su propia detención mediante el comando `stop`.
- **Ciclo de Vida:** El contexto se encarga de gestionar los estados del actor (Started, Running, Stopping, Stopped).
- **Procesamiento de Mensajes:** Una vez que un mensaje llega a la casilla de correo, el contexto de ejecución es el encargado de **llamar al handler específico** para procesar dicho mensaje.

#### Actores: Explicar cuál es la motivación del modelo, características detalladas de la entidad actor y **ciclo de vida de un actor de Actix**.

La principal motivación del modelo de actores es **eliminar los problemas asociados al estado mutable compartido**, como las condiciones de carrera (_race conditions_) y la complejidad de gestionar bloqueos (_locks_). Al no compartir memoria, el modelo facilita el desarrollo de **sistemas complejos, distribuidos y tolerantes a fallos**. Es especialmente útil en entornos donde se requiere una alta consistencia y aislamiento, como en sistemas bancarios o de telecomunicaciones.

Un actor es la primitiva principal de este modelo y posee las siguientes características:

- **Naturaleza liviana:** Los actores son extremadamente ligeros, lo que permite crear **miles o incluso millones de ellos** dentro de un mismo programa, superando la limitación de memoria que imponen los hilos del sistema operativo.
- **Aislamiento total:** Cada actor encapsula su propio comportamiento y **estado privado**, el cual es inaccesible para cualquier otra entidad. La única forma de alterar este estado es mediante el procesamiento de mensajes.
- **Dirección (Address):** Es la referencia necesaria para que otros actores puedan enviarle mensajes. En sistemas distribuidos, esta dirección tiene la capacidad de ser **remota**.
- **Casilla de correo (Mailbox):** Funciona como una cola **FIFO** (primero en entrar, primero en salir) donde se almacenan los mensajes recibidos antes de ser procesados.
- **Procesamiento secuencial:** Aunque el sistema sea altamente concurrente, cada actor procesa **un solo mensaje a la vez**, lo que garantiza la seguridad interna de sus datos sin necesidad de locks.
- **Jerarquía:** Los actores pueden tener una estructura jerárquica donde un actor **supervisor** tiene la capacidad de crear y gestionar actores "hijo".

**Ciclo de Vida de un Actor en Actix**

En el framework Actix, el ciclo de vida de un actor está gestionado de forma automática y consta de los siguientes estados:

1. **Iniciado (Started):** El contexto del actor se crea y se vuelve disponible a través del método `started()`. Este es el punto de entrada inicial tras su creación.
2. **En ejecución (Running):** Es el estado principal del actor tras ejecutarse la fase de inicio. El actor permanece en este estado de forma indefinida mientras procesa mensajes de su casilla de correo.
3. **Parando (Stopping):** El actor entra en esta fase de transición por tres razones principales: si se llama explícitamente a `Context::stop`, si el framework detecta que **ya no existen referencias** (direcciones) al actor, o si no quedan objetos registrados en su contexto.
4. **Detenido (Stopped):** Es el estado final de ejecución. Una vez que el actor llega aquí, se considera finalizado y ya no puede procesar más mensajes.

#### Explicar qué es un **monitor** y en qué se diferencia de un semáforo.

Un **monitor** es una herramienta de sincronización de alto nivel que permite a los hilos tener **exclusión mutua** y suspender su ejecución hasta que una condición específica se vuelva verdadera. Se compone de un nombre, variables internas, procedimientos (rutinas que acceden a dichas variables), una interfaz pública y un conjunto de **variables de condición** que gestionan el sincronismo.

Las diferencias entre ambos mecanismos son las siguientes:

- **Comportamiento de la operación Wait**:
    - En un **semáforo**, la instrucción `wait` puede bloquear o no al proceso, dependiendo de si el contador de recursos disponibles es mayor a cero.
    - En un **monitor**, la instrucción `waitC` **siempre bloquea** al hilo que la ejecuta.
- **Efecto de la operación Signal**:
    - En un **semáforo**, la instrucción `signal` **siempre tiene un efecto** (incrementa el contador de recursos).
    - En un **monitor**, la instrucción `signalC` **no tiene ningún efecto** si la cola de la variable de condición está vacía.
- **Selección de procesos bloqueados:**
    - Cuando un **semáforo** libera un recurso, desbloquea a un proceso de la lista de espera de forma **arbitraria**.
    - En un **monitor**, el `signalC` desbloquea específicamente al proceso que se encuentra en el **tope de la cola (orden FIFO)**.
- **Continuación de la ejecución:**
    - En el **semáforo**, un proceso desbloqueado por un `signal` puede **continuar su ejecución inmediatamente**.
    - En el **monitor**, un proceso que ha sido liberado de una condición de espera debe **aguardar a que el proceso que envió la señal (el señalizador) abandone el monitor** antes de poder retomar su tarea.
- **Almacenamiento de valores:**
    - El **semáforo** funciona como un **contador** (guarda un valor entero no negativo que representa recursos).
    - La variable de condición de un **monitor** **no guarda ningún valor**; simplemente tiene un FIFO asociado para gestionar los hilos en espera.

#### **Semáforo**, sus funciones y variables internas.

Un **semáforo** es una construcción de programación concurrente de alto nivel utilizada como mecanismo de sincronización para controlar el **acceso a un recurso** compartido.

Un semáforo está compuesto por dos campos o variables internas fundamentales:

- **V (Entero no negativo):** Representa la cantidad de **recursos disponibles** en un momento dado. Se inicializa con un valor $k≥0$.
- **L (Set de procesos):** Es un conjunto que almacena los **procesos que están en espera** por conseguir el recurso. Inicialmente, este conjunto está vacío.

Existen variantes específicas, como los semáforos de **System V**, que incluyen variables adicionales como el ID del último proceso que lo utilizó, la cantidad de procesos esperando por el semáforo y la cantidad de procesos esperando a que el valor sea cero.

**Funciones y Operaciones**

El semáforo opera mediante dos funciones principales que deben ser estrictamente **atómicas** (se ejecutan de principio a fin sin interrupciones):

1. **Wait (p)**

Esta operación se utiliza cuando un proceso desea adquirir un recurso:

- **Acción:** Resta 1 al contador de recursos ($V$).
- **Lógica:** Si hay recursos disponibles ($V>0$), el proceso toma uno y continúa su ejecución. Si no hay recursos disponibles ($V≤0$), el proceso se añade a la lista de espera ($L$) y su ejecución se **bloquea**.

2. **Signal (v)**

Esta operación se utiliza cuando un proceso termina de usar un recurso y lo libera:

- **Acción:** Suma 1 al contador de recursos (V).
- **Lógica:** Si la lista de espera (L) está vacía, simplemente se incrementa el contador. Si hay procesos esperando, se elige a uno de ellos de forma **arbitraria**, se lo quita de la lista y se lo pone en estado "listo" para que el sistema operativo lo ejecute y pueda usar el recurso liberado.

**Propiedades e Invariantes**

- **Semáforos binarios:** Si el valor de V solo puede ser 0 o 1, se denomina semáforo binario o **Mutex**, comportándose de forma similar a un lock de escritura.
- **Invariante fundamental:** El valor de V siempre debe ser mayor o igual a cero ($S.V≥0$).
- **Relación matemática:** En cualquier momento, el valor del semáforo es igual al valor inicial ($k$) más la cantidad de señales enviadas menos la cantidad de operaciones wait completadas ($S.V=k+|signal(S)|−|wait(S)|$).

En **Rust**, estas operaciones se implementan a través de métodos como `acquire` (wait) y `release` (signal), permitiendo también el uso del patrón **RAII** mediante el método `access` para liberar el recurso automáticamente al salir del alcance.

#### **Comparar** los modelos de exclusión mutua, fork-join, mensajes y actores. Dar ventajas y desventajas c/u. Dar ejemplos de uso de c/u y por qué convienen.

a. Comparativa de Modelos

|Característica|**Estado Mutable (Exclusión Mutua)**|**Fork-Join**|**Pasaje de Mensajes (Canales)**|**Actores**|
|---|---|---|---|---|
|**Memoria**|**Heap compartido** por todos los hilos.|Heap compartido, pero tareas **aisladas**.|Los hilos **no comparten memoria**.|Estado privado **inaccesible** para otros.|
|**Comunicación**|A través de variables globales protegidas por locks.|Mediante la división y unificación de tareas (_join_).|Envío de datos a través de **canales tipados**.|Intercambio de **mensajes asincrónicos** e inmutables.|
|**Sincronización**|Explícita mediante Mutex, RwLock o Semáforos.|Estructurada; los hilos solo esperan al final del Join.|Implícita en el canal (el receptor espera datos).|Implícita; cada actor procesa **un mensaje por vez**.|
|**Determinismo**|Depende del escenario de ejecución (intercalado).|Es **determinístico**; el resultado no varía por la velocidad.|Depende del orden de llegada al canal o buffer.|Basado en eventos y reactividad.|

b. Ventajas y Desventajas

**Exclusión Mutua (Estado Mutable)**

- **Ventajas:** Ofrece **acceso directo a memoria** con la menor latencia de comunicación y el máximo rendimiento si se gestiona bien.
- **Desventajas:** Alto riesgo de **condiciones de carrera** y _deadlocks_; requiere una gestión compleja de bloqueos que puede impactar la seguridad de los datos.

**Fork-Join**

- **Ventajas:** Implementa **paralelismo estructurado** con balanceo de carga automático (_work stealing_) y está libre de condiciones de carrera.
- **Desventajas:** Las tareas deben ser estrictamente **independientes y aisladas**; existe un costo asociado a la combinación de resultados individuales.

**Pasaje de Mensajes (Canales)**

- **Ventajas:** Elimina errores de memoria compartida al transferir el **ownership** del dato; facilita la sincronización implícita.
- **Desventajas:** Puede introducir _overhead_ al mover o copiar información entre extremos del canal.

**Actores**

- **Ventajas:** Provee **aislamiento total** y alta escalabilidad (son entidades más livianas que los hilos); ideal para sistemas distribuidos y tolerantes a fallos.
- **Desventajas:** Requiere un **framework o runtime externo** (como Actix) y el manejo de lógica asincrónica puede ser más complejo.

c. Ejemplos de Uso y Justificación

- **Exclusión Mutua:** **Motores de videojuegos** (física y gráficos) o **núcleos de sistemas operativos**.
    - _Por qué conviene:_ En estos sistemas, la velocidad de acceso es crítica y los datos deben estar en una **posición de memoria única** para ser leídos/escritos constantemente con mínima latencia.
- **Fork-Join:** **Renderizado de imágenes/video** o algoritmos de ordenamiento como **MergeSort**.
    - _Por qué conviene:_ Son tareas **intensivas de CPU** que se pueden fragmentar fácilmente en subproblemas idénticos, aprovechando todos los núcleos del procesador de forma eficiente.
- **Pasaje de Mensajes:** Sistemas de **productor-consumidor** o flujo de datos entre hilos de procesamiento.
    - _Por qué conviene:_ Permite conectar procesos independientes de forma segura, asegurando que solo una entidad posea el dato en un momento dado.
- **Actores:** **Sistemas bancarios** o **telecomunicaciones** (ej. WhatsApp).
    - _Por qué conviene:_ Estos modelos requieren **consistencia extrema** (cada cuenta bancaria es un actor independiente) y la capacidad de escalar a millones de entidades aisladas en entornos distribuidos.
#### Comparar brevemente el funcionamiento de las primitivas de **semáforos y monitores**.

Tanto los **semáforos** como los **monitores** son herramientas de sincronización de alto nivel diseñadas para gestionar el acceso a recursos y garantizar la exclusión mutua, pero difieren fundamentalmente en su estructura y en cómo manejan el bloqueo y la señalización.

Semáforos

Un semáforo funciona esencialmente como un **contador atómico** (V) acompañado de una lista de procesos en espera (L).

- **Funcionamiento:** Se basa en dos operaciones principales: `wait` (o `p`) y `signal` (o `v`).
- **Gestión del recurso:** El valor del contador representa la cantidad de recursos disponibles. Si un proceso ejecuta `wait` y no hay recursos (V≤0), se bloquea; si hay, decrementa el contador y continúa.
- **Efecto de la señalización:** La operación `signal` **siempre tiene un efecto**: incrementa el contador o despierta a un proceso de la lista de espera.
- **Despertar de procesos:** Cuando un proceso es liberado por un `signal`, se elige de forma **arbitraria** y este puede continuar su ejecución inmediatamente.

Monitores

Un monitor es una construcción que **encapsula variables internas y procedimientos**, garantizando que solo un hilo a la vez pueda ejecutar sus rutinas.

- **Funcionamiento:** Utiliza **variables de condición** (C) para el sincronismo, las cuales no almacenan valores, sino que tienen asociada una cola tipo **FIFO**.
- **Gestión del recurso:** A diferencia del semáforo, la operación `waitC` **siempre bloquea** al proceso que la invoca.
- **Efecto de la señalización:** La operación `signalC` **no tiene efecto** si la cola de la variable de condición está vacía.
- **Despertar de procesos:** Sigue un orden estricto **FIFO** para liberar procesos. Además, un proceso desbloqueado por un `signalC` **debe esperar** a que el proceso que envió la señal abandone el monitor antes de poder continuar.

Comparativa resumida

| Característica                   | **Semáforos**                                  | **Monitores**                                    |
| -------------------------------- | ---------------------------------------------- | ------------------------------------------------ |
| **Componente clave**             | Contador entero (guarda estado).               | Variables de condición (sin valor).              |
| **Comportamiento de** **wait**   | Puede bloquear o no según el contador.         | **Siempre** bloquea al proceso.                  |
| **Comportamiento de** **signal** | Siempre tiene efecto (incrementa o despierta). | No tiene efecto si no hay nadie esperando.       |
| **Orden de desbloqueo**          | Selección arbitraria del proceso.              | Orden **FIFO** (el primero de la cola).          |
| **Prioridad post-señal**         | El proceso despertado sigue inmediatamente.    | El despertado espera a que el señalizador salga. |
	
### Problemas de concurrencia

#### Propiedades

##### Explicar las propiedades Safety y Liveness, y ejemplos de propiedades asociadas a estas.

Propiedades de tipo Safety (Seguridad)

Estas propiedades establecen que **"algo malo nunca sucederá"** o, técnicamente, que deben ser **verdaderas siempre** durante toda la ejecución del programa.

- **Exclusión mutua:** Es la propiedad que garantiza que dos procesos no intercalen sus instrucciones mientras se encuentran en una sección crítica.
- **Ausencia de deadlock (Interbloqueo):** Un sistema debe ser capaz de continuar realizando su tarea y avanzar productivamente. En términos de secciones críticas, si dos procesos desean entrar, **eventualmente alguno de ellos debe tener éxito**; si ninguno pudiera entrar, se violaría esta propiedad de seguridad.

Propiedades de tipo Liveness (Progreso o Vitalidad)

Estas propiedades establecen que **"algo bueno eventualmente sucederá"**, es decir, que deben ser **verdaderas en algún momento** del futuro.

- **Ausencia de starvation (Inanición):** Garantiza que todo proceso que esté listo para utilizar un recurso lo **reciba eventualmente**. Si un proceso desea entrar a su sección crítica, no puede ser ignorado indefinidamente.
- **Fairness (Equidad):** Un escenario se considera _fair_ si, en algún estado, una instrucción que está continuamente habilitada termina apareciendo en algún momento de la ejecución.
- **Progreso en la sección crítica:** Se especifica que una sección crítica debe finalizar eventualmente para permitir que el sistema continúe, mientras que las secciones no críticas no requieren obligatoriamente este progreso (pueden terminar o entrar en bucles infinitos).

Resumen de Diferencias

Mientras que las propiedades de **Safety** se aseguran de que el estado del programa nunca sea inconsistente (como evitar que dos procesos escriban en el mismo sitio a la vez), las de **Liveness** se aseguran de que el programa no se quede "atascado" y que todos los procesos progresen hacia su finalización.

##### Definir el problema de la Sección Crítica (SC). Explicar las 3 propiedades de corrección que debe cumplir.

El problema de la **Sección Crítica (SC)** surge fundamentalmente en el modelo de **estado mutable compartido**, donde varios procesos pueden modificar variables globales de un programa. Si no se administra correctamente el acceso a estas variables, el programa puede quedar en un **estado inconsistente**.

En este contexto, un proceso se visualiza como un bucle infinito dividido en secciones **críticas** (donde accede al recurso compartido) y **no críticas**. El desafío consiste en marcar estas regiones de código de modo que **no puedan superponerse** en la ejecución al mismo tiempo.

Para que una solución al problema de la sección crítica se considere correcta, debe cumplir con las siguientes **tres propiedades**:

1. Exclusión Mutua (Mutual Exclusion)

Es una propiedad de tipo **Safety** (seguridad) que establece que **"algo malo nunca sucederá"**.

- **Definición:** Garantiza que dos o más procesos no intercalen sus instrucciones mientras se encuentran dentro de sus respectivas secciones críticas.
- **Funcionamiento:** Si un proceso está ejecutando su SC, cualquier otro proceso que intente entrar a la suya será obligado a esperar hasta que la primera ejecución haya terminado.

2. Ausencia de Deadlock (Interbloqueo)

También clasificada como una propiedad de tipo **Safety**.

- **Definición:** Establece que un sistema debe ser capaz de avanzar productivamente en su tarea.
- **En la SC:** Si dos o más procesos desean entrar a su sección crítica, el mecanismo de control debe garantizar que **eventualmente alguno de ellos tenga éxito**. Si existiera una situación donde todos quieren entrar pero ninguno puede hacerlo, se produciría un _deadlock_.

3. Ausencia de Starvation (Inanición)

Es una propiedad de tipo **Liveness** (vitalidad o progreso), lo que significa que **"algo bueno eventualmente sucederá"**.

- **Definición:** Garantiza que todo proceso que esté listo para usar un recurso lo reciba en algún momento del futuro.
- **En la SC:** Si un proceso manifiesta su intención de entrar a la sección crítica, el sistema debe asegurar que **eventualmente podrá entrar**. Esto evita que un proceso sea ignorado indefinidamente mientras otros entran y salen de la SC.

Como condición adicional para el correcto funcionamiento, las fuentes mencionan que la **sección crítica debe progresar**, es decir, debe finalizar eventualmente para liberar el recurso, mientras que las secciones no críticas no requieren obligatoriamente este progreso.

##### Describir y comparar los efectos negativos que tienen en los programas concurrentes: Deadlocks, Race conditions, Starvation, Busy wait.

En el desarrollo de programas concurrentes, surgen diversos efectos negativos que afectan tanto la **corrección** (si el programa hace lo que debe) como el **rendimiento** del sistema. Estos problemas se clasifican principalmente en fallos de seguridad (**Safety**), vitalidad (**Liveness**) y eficiencia.

1. Deadlocks (Interbloqueos)

El **deadlock** es una propiedad de tipo **Safety** que ocurre cuando un sistema no puede realizar ninguna tarea productiva porque los procesos se bloquean entre sí de forma circular.

- **Descripción:** En el contexto de una sección crítica, se produce si dos o más procesos desean entrar, pero ninguno tiene éxito, quedando todos en espera indefinida. Es común en modelos de estado mutable donde se utilizan **locks** de forma incorrecta.
- **Efectos Negativos:** El programa deja de progresar, quedando "congelado".
- **Detección y Prevención:** Se puede detectar mediante un **grafo de uso de recursos** buscando ciclos. Para prevenirlo, se utilizan algoritmos basados en timestamps como **wait-die** (el proceso más joven aborta) o **wound-wait** (el proceso más viejo fuerza el aborto del más nuevo).

2. Race Conditions (Condiciones de Carrera)

Las **condiciones de carrera** ocurren cuando el resultado de un programa depende del orden impredecible en que se intercalan las instrucciones de diferentes procesos.

- **Descripción:** Es el riesgo principal del modelo de **estado mutable compartido**, donde varios hilos acceden y modifican la misma posición de memoria simultáneamente.
- **Efectos Negativos:** El sistema puede quedar en un **estado inconsistente**, arrojando resultados erróneos o corruptos que varían en cada ejecución.
- **Solución:** Se evita mediante mecanismos de sincronización como **Mutex**, semáforos o adoptando modelos que aíslan el estado, como el de **Actores** o **Fork-Join**.

3. Starvation (Inanición)

La **inanición** es una violación de una propiedad de tipo **Liveness**, la cual dicta que "algo bueno debe suceder eventualmente".

- **Descripción:** Sucede cuando un proceso que está listo para utilizar un recurso nunca lo recibe porque otros procesos lo acaparan constantemente o el sistema lo ignora indefinidamente.
- **Efectos Negativos:** Aunque el sistema en general parezca estar funcionando (no hay deadlock), uno o más procesos específicos nunca logran terminar su tarea.
- **Diferencia clave:** A diferencia del deadlock, donde todos los implicados están bloqueados, en la inanición el sistema puede seguir progresando, pero es injusto con ciertos procesos.

4. Busy Wait (Espera Activa)

El **Busy Wait** no es necesariamente un error de lógica de corrección, sino un problema grave de **eficiencia de recursos**.

- **Descripción:** Ocurre cuando un hilo se mantiene en un **bucle infinito** verificando constantemente una condición (como si un lock se liberó) en lugar de suspenderse y ceder el procesador.
- **Efectos Negativos:** Consume el **100% de un núcleo de CPU** sin realizar trabajo útil, quitando recursos valiosos a otros hilos que sí podrían progresar.
- **Alternativa:** Se debe utilizar el **bloqueo pasivo**, donde el hilo se "duerme" y es despertado por el sistema operativo mediante señales o variables de condición (**Condvars**) cuando la condición cambia.

Comparativa de los efectos

| Problema           | Tipo de Propiedad | Efecto Principal                              | Impacto en el Programa                             |
| ------------------ | ----------------- | --------------------------------------------- | -------------------------------------------------- |
| **Deadlock**       | Safety            | Bloqueo circular mutuo.                       | El programa se detiene por completo.               |
| **Race Condition** | Safety            | Dependencia del escenario de ejecución.       | Datos corruptos y resultados inconsistentes.       |
| **Starvation**     | Liveness          | Falta de progreso para un proceso específico. | Algunos procesos nunca terminan su ejecución.      |
| **Busy Wait**      | Rendimiento       | Consumo inútil de ciclos de CPU.              | Desperdicio masivo de recursos y lentitud general. |
#### Deadlocks

##### ¿Qué es un deadlock? Explique sus consecuencias en un programa concurrente. Mencione qué complicaciones adicionales tienen los deadlocks en ambientes de concurrencia distribuida. Explique a detalle mecanismos de prevención de deadlocks en ambientes distribuidos.

Un **deadlock** (o interbloqueo) es una falla en la propiedad de **Safety** de un programa concurrente que ocurre cuando un sistema no puede continuar realizando su tarea ni avanzar productivamente. Técnicamente, se produce cuando existe un **bloqueo circular** entre procesos que esperan por recursos que otros poseen, quedando todos en un estado de espera indefinida.

**Consecuencias en un programa concurrente**

La consecuencia principal de un deadlock es que el programa se "congela" y deja de progresar.

- **Inactividad:** Los procesos involucrados no pueden ejecutar ninguna instrucción atómica adicional que los acerque a su finalización.
- **Violación de Corrección:** Se incumple la especificación de que, si varios procesos desean entrar a una sección crítica, al menos uno debe tener éxito.
- **Desperdicio de Recursos:** Los recursos (como memoria o locks) quedan retenidos por los procesos bloqueados, volviéndose inaccesibles para el resto del sistema.

**Complicaciones en ambientes distribuidos**

En entornos distribuidos, la detección y gestión de deadlocks es más compleja debido a la naturaleza de la comunicación y la falta de un estado global compartido:

- **Falsos Deadlocks:** En algoritmos de detección centralizados, los mensajes de obtención o liberación de recursos pueden llegar al coordinador de forma desordenada o con retraso, provocando que el coordinador detecte ciclos en el grafo de recursos que en realidad ya no existen.
- **Ausencia de Reloj Global:** Es difícil determinar el orden exacto de los eventos. Se requiere el uso de **timestamps globales** (como el algoritmo de relojes de Lamport) para ordenar los mensajes y evitar inconsistencias.
- **Fallos Independientes:** Las entidades (procesos) pueden fallar de forma aleatoria, y los errores de comunicación deben ser manejados transparentemente para no confundir una caída de red con un bloqueo.

**Mecanismos de prevención en ambientes distribuidos**

Para prevenir deadlocks antes de que ocurran en sistemas distribuidos, se asigna a cada transacción un **timestamp único y global** al momento de su inicio. Basándose en la "edad" de la transacción, se aplican dos estrategias principales:

1. Algoritmo Wait-Die (Esperar-Morir)

Este mecanismo se basa en que los procesos más jóvenes nunca hagan esperar a los más viejos:

- Cuando un proceso intenta bloquear un recurso que ya tiene otro proceso, se comparan sus timestamps.
- **Si el demandante es más viejo** (timestamp menor): Se le permite **esperar** a que el recurso se libere.
- **Si el demandante es más joven** (timestamp mayor): El proceso se **aborta** (muere) para evitar un posible ciclo, y la transacción debe reiniciarse más tarde.

2. Algoritmo Wound-Wait (Herir-Esperar)

Es la contraparte del anterior y prioriza la supervivencia de los procesos más viejos de forma más agresiva:

- Cuando ocurre una disputa por un recurso:
- **Si el demandante es más viejo** (timestamp menor): El proceso demandante **"hiere" (aborta)** la transacción del proceso más joven que posee el recurso, obligándolo a liberar el recurso para que el viejo pueda tomarlo.
- **Si el demandante es más joven** (timestamp mayor): El proceso simplemente **espera** a que el proceso viejo termine de usar el recurso.

Ambos mecanismos garantizan que no se formen ciclos de espera, ya que las transacciones solo pueden esperar o abortar siguiendo un orden estricto basado en su tiempo de creación.

##### Explicar algoritmos de detección de deadlocks distribuidos, y diferencias entre prevención y detección de deadlocks.

En los sistemas distribuidos, el manejo de **deadlocks** (interbloqueos) es fundamental para garantizar la propiedad de **Safety**, la cual asegura que el sistema siempre pueda avanzar productivamente. A continuación, se detallan los algoritmos de detección y las diferencias clave entre las estrategias de prevención y detección.

**Algoritmos de Detección de Deadlocks Distribuidos**

La detección busca identificar la existencia de un **bloqueo circular** en el grafo de uso de recursos una vez que este ya ha ocurrido.

- **Algoritmo Centralizado:**
    - Un proceso **coordinador** mantiene un **grafo de uso de recursos** global.
    - Cada vez que un proceso obtiene o libera un recurso, envía un mensaje al coordinador para que actualice el grafo.
    - El coordinador analiza el grafo en busca de **ciclos**; si encuentra uno, existe un deadlock.
    - **Problema de los "Falsos Deadlocks":** Debido a que los mensajes pueden llegar al coordinador de forma desordenada, este podría detectar un ciclo que en la realidad ya no existe. Para mitigar esto, se suelen utilizar **timestamps globales** (como los de Lamport) para ordenar cronológicamente los mensajes.
- **Algoritmo Distribuido (Mensaje de Sonda):**
    - No depende de un coordinador central. Cuando un proceso se bloquea esperando un recurso, inicia la detección enviando un **"probe message"** (mensaje de sonda) al proceso que posee dicho recurso.
    - Este mensaje contiene tres datos: el **ID del proceso bloqueado**, el **ID del proceso que envía el mensaje** y el **ID del destinatario**.
    - Si el receptor también está esperando por otro recurso, actualiza los IDs del mensaje y lo reenvía.
    - Si el mensaje de sonda eventualmente **regresa al proceso original** que inició la consulta, se confirma la existencia de un ciclo y, por lo tanto, de un deadlock.

**Diferencias entre Prevención y Detección**

La principal distinción radica en **cuándo** actúa el sistema para manejar el fallo de seguridad.

| Característica                   | **Prevención de Deadlocks**                                                                                                 | **Detección de Deadlocks**                                                                                  |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| **Momento de acción**            | Actúa **antes** de que ocurra el bloqueo, impidiendo que se cumplan las condiciones para un ciclo.                          | Actúa **después** de que el bloqueo ha ocurrido, buscando ciclos en el sistema.                             |
| **Mecanismo principal**          | Utiliza **timestamps únicos globales** para establecer un orden de prioridad entre transacciones.                           | Utiliza un **grafo de uso de recursos** o el paso de **mensajes de sonda** para hallar ciclos.              |
| **Estrategias comunes**          | **Wait-die** (el más joven aborta) y **Wound-wait** (el más viejo aborta al más joven).                                     | Algoritmos centralizados con coordinador o algoritmos distribuidos de detección de ciclos.                  |
| **Consecuencia para el proceso** | Puede obligar a un proceso a **abortar preventivamente** (rollback) aunque no haya un deadlock real, para evitar el riesgo. | El proceso permanece **bloqueado** hasta que el algoritmo detecta el ciclo y se toma una medida correctiva. |
| **Complejidad**                  | Requiere una gestión estricta de tiempos de inicio de transacciones.                                                        | Puede generar "falsos deadlocks" si la comunicación entre nodos es lenta o desordenada.                     |

En resumen, mientras que la **detección** permite que el sistema funcione libremente hasta que se topa con un problema, la **prevención** impone reglas de prioridad basadas en la "edad" de las transacciones (timestamps) para garantizar matemáticamente que un ciclo de espera circular nunca llegue a formarse.

##### Diagramar un ejemplo de detección de deadlock y explicarlo paso a paso.

Para diagramar y explicar la detección de un **deadlock** (interbloqueo), utilizaremos el **Algoritmo Distribuido de Mensaje de Sonda (Probe Message)**, el cual es ideal para identificar ciclos de espera circular sin necesidad de un coordinador central.

Escenario de Ejemplo

Imaginemos un sistema con tres procesos (P1,P2,P3) donde existe una dependencia circular de recursos:

1. **P1** tiene el Recurso A y está bloqueado esperando el Recurso B (que tiene **P2**).
2. **P2** tiene el Recurso B y está bloqueado esperando el Recurso C (que tiene **P3**).
3. **P3** tiene el Recurso C y está bloqueado esperando el Recurso A (que tiene **P1**).

**Diagrama de espera (Grafo de uso de recursos):** `P1` → `P2` → `P3` → `P1` (Ciclo)

El algoritmo utiliza un mensaje de sonda con la estructura: `(ID_proceso_bloqueado, ID_emisor_actual, ID_destinatario)`.

Paso 1: Iniciación por P1

Cuando **P1** nota que debe esperar por un recurso que posee **P2**, inicia la detección enviando una sonda al poseedor del recurso.

- **Mensaje enviado:** `(1, 1, 2)`
    - _ID Bloqueado:_ 1 (P1 inició la sonda)
    - _ID Emisor:_ 1 (P1 envía el mensaje)
    - _ID Destinatario:_ 2 (P2 recibe el mensaje)

Paso 2: Propagación por P2

**P2** recibe la sonda `(1, 1, 2)`. Como **P2** también está bloqueado esperando un recurso que posee **P3**, actualiza la información del emisor y el destinatario y reenvía el mensaje.

- **Mensaje enviado:** `(1, 2, 3)`
    - _ID Bloqueado:_ 1 (Sigue siendo P1 quien inició el ciclo)
    - _ID Emisor:_ 2 (Ahora el emisor es P2)
    - _ID Destinatario:_ 3 (P3 recibe el mensaje)

Paso 3: Propagación por P3

**P3** recibe la sonda `(1, 2, 3)`. Dado que **P3** está esperando el recurso que tiene **P1**, actualiza nuevamente los campos de envío y dirige el mensaje al poseedor del recurso.

- **Mensaje enviado:** `(1, 3, 1)`
    - _ID Bloqueado:_ 1
    - _ID Emisor:_ 3
    - _ID Destinatario:_ 1

Paso 4: Detección del Deadlock

**P1** recibe el mensaje de sonda `(1, 3, 1)`. Al procesar el mensaje, **P1** identifica que el _ID del proceso que se bloquea_ (el primer campo) coincide con su propio ID.

- **Resultado:** Se confirma matemáticamente que existe un **ciclo en el grafo de recursos**. El sistema ha detectado el **deadlock** y puede proceder a tomar medidas correctivas (como abortar una transacción).

Como alternativa, existe el **Algoritmo Centralizado**, donde un **coordinador único** mantiene el grafo global de uso de recursos. En ese caso, cada vez que P1, P2 o P3 solicitan o liberan un recurso, envían un mensaje al coordinador, quien actualiza el grafo y busca ciclos constantemente. Sin embargo, este modelo puede generar "falsos deadlocks" si los mensajes llegan desordenados debido a la latencia de la red, problema que suele mitigarse usando **timestamps globales**.

### Sistemas distribuidos

#### Conceptos básicos y Entidades

##### Qué es entidad y sus capacidades. Conocimiento distribuido.

En el contexto de los ambientes distribuidos, una **entidad** se define como la **unidad de cómputo fundamental** de dicho entorno. Dependiendo del nivel de abstracción, una entidad puede ser un proceso, un procesador o, en niveles más altos, un **Actor**.

Cada entidad en un sistema distribuido cuenta con cuatro capacidades básicas fundamentales:

1. **Memoria Local:** Tiene acceso de lectura y escritura a una memoria propia que **no es compartida** con otras entidades. En ella mantiene un registro de estado ($status(x)$) y un registro de valor de entrada ($value(x)$).
2. **Procesamiento Local:** Capacidad para realizar cálculos y ejecutar operaciones internamente.
3. **Comunicación:** Puede preparar, transmitir y recibir mensajes hacia y desde otras entidades.
4. **Gestión del Tiempo:** Capacidad para setear y resetear un **reloj local**.

Además, las entidades son de naturaleza **reactiva**, lo que significa que solo actúan en respuesta a **eventos externos** como la llegada de un mensaje, la activación de una alarma del reloj o un impulso espontáneo.

##### Regla, acción, comportamiento y conocimiento de entidades.

En los ambientes distribuidos, el funcionamiento de las **entidades** (unidades de cómputo como procesos o actores) se define a través de su capacidad de reaccionar a eventos mediante reglas y acciones predefinidas.

Acción y Regla

Las entidades son de naturaleza **reactiva**, lo que significa que solo actúan en respuesta a eventos externos, como la llegada de un mensaje, la activación de una alarma del reloj o un impulso espontáneo.

- **Acción:** Es una secuencia finita e indivisible de operaciones. Se considera **atómica** porque, una vez iniciada, se ejecuta de principio a fin sin interrupciones.
- **Regla:** Es la relación lógica que determina qué hacer ante un estímulo. Establece que, dado un **evento** específico y el **estado** actual de la entidad, se debe ejecutar una **acción** determinada (estado×evento→acción).

Comportamiento

El **comportamiento** de una entidad, denotado como B(x), es el **conjunto de todas las reglas** que dicha entidad obedece.

- **Unicidad:** Para cada combinación posible de estado y evento, debe existir una única regla aplicable.
- **Protocolo:** El comportamiento es, en esencia, el **algoritmo distribuido** o protocolo que sigue la entidad.
- **Homogeneidad:** Se dice que el comportamiento colectivo del sistema es **homogéneo** cuando todas las entidades que lo componen siguen exactamente el mismo conjunto de reglas ($B(x)=B(y)$). Todo comportamiento colectivo tiene la propiedad de poder ser transformado en uno homogéneo.

Conocimiento de las Entidades

El conocimiento que posee una entidad se divide en lo que mantiene de forma privada y lo que conoce sobre la estructura del sistema en el que habita.

1. **Conocimiento Local:** Comprende el contenido de la memoria local de la entidad (sus registros de estado y valor) y toda la información que pueda derivar de ellos. En ausencia de fallas, este conocimiento es persistente y no se pierde.
2. **Conocimiento sobre la Red:** Las fuentes clasifican esta información en tres niveles:
    - **Información Métrica:** Datos numéricos sobre la red, como la cantidad total de nodos (n), el número de arcos (m) o el diámetro del grafo.
    - **Propiedades Topológicas:** Conocimiento sobre la forma o características estructurales de la red, como saber si es un anillo o si es un grafo acíclico.
    - **Mapas Topológicos:** Un mapa detallado de la vecindad de la entidad (hasta una distancia d), como puede ser una matriz de adyacencia del grafo.

Este sistema garantiza el **determinismo** en la ejecución: si una entidad recibe el mismo evento encontrándose en el mismo estado interno en dos ejecuciones distintas, su nuevo estado resultante será idéntico en ambos casos.

##### Enumerar posibles eventos en un ambiente distribuido. Explicar cómo se calcula el costo y complejidad en un ambiente distribuido y comparar el cómputo que hay que hacer en un sistema centralizado.

En un ambiente distribuido, las entidades (unidades de cómputo como procesos o actores) operan bajo un modelo **reactivo**, lo que significa que su estado solo cambia y sus acciones solo se ejecutan en respuesta a estímulos específicos denominados eventos externos.

**Posibles eventos en un ambiente distribuido**

Existen tres tipos principales de eventos externos que disparan el comportamiento de una entidad:

1. **Llegada de un mensaje:** La recepción de una secuencia finita de bits enviada por otra entidad a través de la red.
2. **Activación del reloj (alarma):** El aviso de un temporizador o reloj local que la entidad ha seteado previamente.
3. **Impulso espontáneo:** Un evento que ocurre de forma interna u original, sin ser una respuesta directa a otro evento del sistema.

A excepción del impulso espontáneo, todos los eventos se consideran generados dentro de los límites del sistema.

**Cálculo de costo y complejidad**

Para comparar la eficiencia de los algoritmos distribuidos, se utilizan métricas que miden tanto el uso de la red como el rendimiento temporal:

- **Complejidad de Mensajes (M):** Se calcula midiendo la **cantidad total de transmisiones** o mensajes necesarios para que el protocolo finalice. También se evalúa la carga de transmisión y la carga de trabajo individual por entidad.
- **Complejidad de Tiempo:**
    - **Tiempo total:** Es la duración real de la ejecución del protocolo desde su inicio hasta que todas las entidades terminan sus acciones.
    - **Tiempo ideal:** Es una medida teórica calculada bajo condiciones controladas, como **delays de comunicación unitarios** (cada mensaje tarda exactamente una unidad de tiempo) y **relojes sincronizados**.

**Comparación con el cómputo en sistemas centralizados**

El cómputo en un sistema distribuido presenta diferencias fundamentales respecto a uno centralizado en cuanto a su ejecución y costos asociados:

- **Naturaleza del Cómputo:**
    - **Sistemas Centralizados:** El cómputo suele ser un conjunto de instrucciones secuenciales ejecutadas en un solo procesador que accede a una memoria principal. La complejidad se mide principalmente en ciclos de CPU y uso de memoria local.
    - **Sistemas Distribuidos:** El cómputo está fragmentado en múltiples entidades independientes con **memoria local no compartida**. El resultado del sistema depende de la interacción y el intercambio de mensajes entre estas entidades.
- **Mecanismos de Sincronización:**
    - **Centralizado:** Se utilizan variables compartidas protegidas por **locks o semáforos**. El costo principal es el tiempo de espera por recursos locales.
    - **Distribuido:** El costo se desplaza hacia la **comunicación**. Para lograr objetivos como la exclusión mutua, se requieren algoritmos complejos (como Ricart-Agrawala o Token Ring) que dependen del envío de múltiples mensajes "OK" o de la circulación de un token, lo que eleva la complejidad de red significativamente en comparación con un simple lock local.
- **Gestión del Estado:**
    - **Centralizado:** Existe un estado global único y predecible; ante una misma entrada y escenario, se obtiene la misma salida.
    - **Distribuido:** No existe un reloj global ni un estado único instantáneo. La corrección depende de la secuencia de eventos y los delays de comunicación, los cuales pueden variar en cada ejecución, generando diferentes resultados posibles ante el mismo protocolo.

#### Exclusión mutua distribuida y Elección de Líder

##### Explicar los algoritmos de elección del líder (Bully y Ring).

Los **algoritmos de elección** se utilizan en sistemas distribuidos cuando se requiere un coordinador para cumplir un rol especial (como en la exclusión mutua distribuida). Para su funcionamiento, se asume que cada proceso posee un **ID único**, se ejecuta un solo proceso por máquina y todos conocen los IDs de los demás procesos.

**Algoritmo Bully (Matón)**

Este algoritmo se basa en la premisa de que el proceso con el **ID más alto** siempre gana la elección. El proceso se desarrolla de la siguiente manera:

1. **Inicio:** Cuando un proceso P detecta que el coordinador no responde, inicia el proceso de elección enviando un mensaje de `ELECTION` a todos los procesos que tengan un **número de ID mayor** al suyo.
2. **Respuesta:**
    - Si **nadie responde**, el proceso P gana automáticamente la elección y se convierte en el nuevo coordinador.
    - Si **algún proceso con ID mayor contesta**, dicho proceso asume la responsabilidad de continuar con la elección y el proceso P original finaliza su participación.
3. **Anuncio:** Una vez que se determina el ganador, el nuevo coordinador se anuncia enviando un mensaje de `COORDINATOR` a todos los demás.

**Algoritmo Ring (Anillo)**

Este algoritmo utiliza una estructura lógica de anillo donde cada proceso **conoce a su sucesor**.

1. **Detección:** Cuando un proceso nota la falla del coordinador, construye un mensaje de `ELECTION` que incluye su propio número de proceso y lo envía a su sucesor.
2. **Circulación:** Cada proceso que recibe el mensaje **agrega su propio ID a la lista** dentro del mensaje y lo reenvía al siguiente sucesor en el anillo.
3. **Selección:** Cuando el mensaje regresa al proceso que inició la elección (completando el círculo), este revisa la lista acumulada. El proceso con el **número de ID más alto de la lista** es elegido como el nuevo coordinador.
4. **Finalización:** El iniciador cambia el tipo de mensaje a `COORDINATOR` y lo envía a través del anillo para informar a todos quién es el nuevo líder y actualizar la estructura del anillo. El mensaje se elimina una vez que termina de circular.

##### Exclusión mutua en ambiente distribuido, diagrama y problemas.

La **exclusión mutua en ambientes distribuidos** es la extensión del problema de la sección crítica a sistemas donde los procesos no comparten memoria física y deben coordinarse mediante el paso de mensajes. Su objetivo es garantizar que, en un sistema de procesos independientes, solo uno pueda acceder a un recurso compartido (como una impresora o una base de datos) a la vez.

A continuación, se detallan los tres algoritmos principales para resolver este problema, sus diagramas lógicos y los inconvenientes asociados.

1. Algoritmo Centralizado

Es el modelo más simple, donde se designa a un proceso específico para gestionar el acceso.

- **Funcionamiento (Diagrama paso a paso):**
    1. Se elige un proceso como **coordinador**.
    2. Un proceso que desea entrar a la Sección Crítica (SC) envía un mensaje de **solicitud** al coordinador.
    3. **Si la SC está libre:** El coordinador responde inmediatamente con un mensaje de **OK**.
    4. **Si la SC está ocupada:** El coordinador no responde (o encola la petición) hasta que el proceso actual libere la SC.
    5. Al terminar, el proceso envía un mensaje de **liberación** al coordinador para que este pueda autorizar al siguiente en la cola.
- **Problemas:** El coordinador es un **punto único de falla**; si este cae, todo el sistema de exclusión mutua colapsa. Además, puede convertirse en un cuello de botella en sistemas con alta demanda.

2. Algoritmo Distribuido (Ricart–Agrawala)

Este algoritmo elimina la necesidad de un coordinador central, basándose en el consenso de todos los participantes y el uso de **timestamps** para determinar prioridades.

- **Funcionamiento (Diagrama paso a paso):**
    1. Un proceso que quiere entrar a la SC construye un mensaje con su **ID, el nombre de la sección y un timestamp** (marca de tiempo global), y lo envía a **todos** los demás procesos.
    2. **Reglas de respuesta del receptor:**
        - Si no le interesa entrar a la SC: Envía un **OK** inmediatamente.
        - Si ya está dentro de la SC: No responde y **encola** la petición.
        - Si también quiere entrar: Compara los timestamps; el que tenga el **valor menor** (el más antiguo) gana y el otro encola la petición.
    3. El proceso solicitante **solo entra a la SC** cuando recibe el **OK de todos** los demás procesos del sistema.
- **Problemas:** Tiene una alta **complejidad de mensajes** ($2*(n−1)$ mensajes por cada entrada a la SC). Además, la falla de cualquier nodo detiene el sistema, ya que se requiere la respuesta de todos para progresar.

3. Algoritmo Token Ring

Utiliza una estructura de **anillo lógico** para organizar el paso de permisos.

- **Funcionamiento (Diagrama paso a paso):**
    1. Los procesos se conectan punto a punto formando un círculo.
    2. Un único mensaje especial, llamado **token**, circula constantemente por el anillo.
    3. **Acceso:** Solo el proceso que posee el token en un momento dado tiene permiso para entrar a la SC.
    4. **Salida:** Al terminar, el proceso pasa el token a su sucesor. No puede volver a entrar a la SC hasta que el token complete una vuelta entera.
- **Problemas:** Si el token se pierde (por una falla de red o de un nodo), se debe iniciar un proceso complejo para regenerarlo. Si un proceso falla, el anillo debe reconstruirse para saltar el nodo caído.

**Problemas Generales en Ambientes Distribuidos**

Más allá de las fallas de cada algoritmo, la exclusión mutua distribuida enfrenta desafíos estructurales debido a la naturaleza de la red:

- **Ausencia de Estado Global:** No hay una memoria compartida ni un reloj global perfecto, lo que obliga a usar algoritmos de sincronización de tiempo como los de **Lamport** para ordenar los eventos.
- **Mensajes Desordenados y Falsos Deadlocks:** Los retrasos en la red pueden causar que los mensajes de solicitud y liberación lleguen en el orden incorrecto al destino. Esto puede provocar que un sistema de detección crea que hay un ciclo de interbloqueo (**falso deadlock**) cuando en realidad el recurso ya fue liberado pero el mensaje de aviso aún no llegó.
- **Confiabilidad de la Comunicación:** Se requiere que la capa de red garantice que los mensajes lleguen intactos, ya que la pérdida de un "OK" o un "Token" puede dejar a un proceso bloqueado indefinidamente esperando un recurso que está libre.

### Transacciones

#### Precondiciones de transacciones (storage estable, tiempos de comunicación finitos).

Para el correcto funcionamiento de las **transacciones** en ambientes distribuidos, el sistema debe operar bajo un modelo que garantice ciertas precondiciones fundamentales sobre el almacenamiento y la red.

**Precondiciones de Transacciones**

Las bases para implementar un sistema de transacciones confiable son:

- **Storage Estable (Almacenamiento Estable):** Esta condición asegura que los datos críticos no se pierdan ante fallos del sistema.
    - Se implementa generalmente mediante **discos u otros medios de almacenamiento durables**.
    - Su característica principal es que la probabilidad de perder los datos almacenados en él es **extremadamente pequeña**.
    - Es vital para la propiedad de **Durabilidad (ACID)**, que establece que una vez que se confirman (_commit_) los cambios, estos deben ser permanentes.
- **Tiempos de Comunicación Finitos (Axioma de Delays):** Para que los protocolos de coordinación (como el _Commit en dos fases_) puedan progresar, se asume que la comunicación entre los nodos es viable.
    - Se establece que, en ausencia de fallas, los **delays de comunicación tienen una duración finita**.
    - En variantes más estrictas, se puede hablar de **delays acotados**, donde existe una constante máxima de tiempo para la entrega de mensajes, o incluso **delays unitarios**, donde cada mensaje tarda exactamente una unidad de tiempo.

**Otras condiciones del modelo**

Además de las anteriores, el modelo de transacciones asume lo siguiente:

1. **Independencia de procesos:** El sistema está conformado por procesos independientes que pueden **fallar de forma aleatoria**.
2. **Transparencia de red:** Los errores que puedan ocurrir en la comunicación son **manejados transparentemente** por la capa de comunicación del sistema.
3. **Uso de Logs:** Para garantizar la atomicidad y permitir recuperaciones (_rollbacks_), se utilizan mecanismos como el **Writeahead Log**, donde los cambios se anotan en una lista antes de modificar los archivos finales.

#### Explicar el commit en 2 fases, dar ventajas y desventajas. Explicar concurrencia optimista, dar un caso de uso y por qué conviene.

El **commit en dos fases** y la **concurrencia optimista** son mecanismos fundamentales para garantizar la integridad de los datos en sistemas de transacciones distribuidas.

**Commit en dos fases (Two-Phase Commit)**

Es un protocolo de coordinación utilizado para asegurar que una transacción se complete de forma atómica en todos los procesos participantes. El proceso es liderado por un **coordinador** y se divide en las siguientes etapas:

- **Fase 1 (Preparación):**
    1. El coordinador registra la instrucción _prepare_ en su log y envía un mensaje de _prepare_ a todos los demás procesos.
    2. Los participantes reciben el mensaje, escriben _ready_ en sus propios logs y envían una confirmación de _ready_ al coordinador.
- **Fase 2 (Compromiso):**
    1. Si todos están listos, el coordinador aplica los cambios y envía el mensaje de _commit_ a los procesos.
    2. Los procesos receptores escriben _commit_ en sus logs y responden con un mensaje de _finished_ al coordinador.

**Ventajas y Desventajas**

- **Ventajas:** Su principal beneficio es garantizar las propiedades **ACID** (Atomicidad, Consistencia, Aislamiento y Durabilidad) en entornos distribuidos, asegurando que los cambios sean permanentes una vez confirmados. Además, el uso de **logs** permite realizar un _rollback_ (deshacer cambios) si la transacción se aborta antes del commit.
- **Desventajas:** Es un protocolo dependiente de la disponibilidad de todos los nodos y del coordinador. Si bien las fuentes no listan desventajas puntuales en una tabla, mencionan que el sistema debe manejar transparentemente los errores de comunicación y fallos aleatorios de los procesos para no quedar bloqueado.

---

**Concurrencia Optimista**

Este modelo parte de la premisa de que los conflictos entre transacciones son poco frecuentes. El proceso modifica los archivos **sin aplicar controles o bloqueos iniciales**, esperando que no surjan disputas por los datos. Al momento de intentar el _commit_, el sistema verifica si otras transacciones modificaron los mismos archivos; si se detecta un conflicto, la transacción se aborta.

Ventajas y Desventajas

- **Ventajas:** Es un modelo **libre de deadlocks** (interbloqueos) y favorece significativamente el **paralelismo**, ya que los hilos no pierden tiempo esperando por locks de exclusión mutua.
- **Desventajas:** En escenarios de **alta carga** o alta contención de datos, tener que abortar y **rehacer todo el trabajo** desde cero puede resultar extremadamente costoso en términos de rendimiento.

Caso de uso y conveniencia

Un caso de uso ideal son los **sistemas con baja probabilidad de conflicto**, como una aplicación de edición de perfiles de usuario donde es muy raro que dos procesos intenten modificar exactamente el mismo registro al mismo tiempo.

- **Por qué conviene:** Conviene porque elimina el _overhead_ (carga extra) de gestionar locks complejos (como el _two-phase locking_) y evita que los procesos se bloqueen innecesariamente, permitiendo que el sistema procese muchas más operaciones por segundo mientras no existan colisiones reales.

#### Control de concurrencia: Definir 2-phase locking, timestamps y concurrencia optimista. Situación de supermercado con mucha contención en hora pico: ventajas y desventajas para los 3 tipos de control, elegir uno y justificar.

Para gestionar el acceso a recursos en sistemas de transacciones, existen diversos modelos de control de concurrencia con comportamientos distintos ante la carga de trabajo.

Definiciones de Control de Concurrencia

- **2-Phase Locking (Bloqueo de dos fases):** Es un protocolo que garantiza la serializabilidad dividiendo la gestión de locks en dos etapas: una **fase de expansión**, donde la transacción adquiere todos los locks necesarios, y una **fase de contracción**, donde los libera sin poder solicitar nuevos. Existe una variante llamada _Strict 2PL_ donde la liberación ocurre recién después del commit.
- **Timestamps (Marcas de tiempo):** Asigna a cada transacción un identificador único global para establecer un orden cronológico. Cada archivo registra el timestamp de su última lectura y escritura; si una transacción intenta operar y su timestamp es menor al del archivo (está "fuera de orden"), la transacción se aborta.
- **Concurrencia Optimista:** Este modelo permite que los procesos modifiquen archivos sin controles ni bloqueos previos, bajo la premisa de que no habrá conflictos. Al momento del commit, se realiza una validación; si otra transacción modificó los mismos datos, la actual se aborta y debe reiniciarse.

---

Análisis: Supermercado en Hora Pico (Alta Contención)

En un supermercado durante la hora pico, existe una **alta contención**, lo que significa que múltiples cajas intentan actualizar el stock de los mismos productos populares o acceder a los mismos registros de cuenta simultáneamente.

|Modelo|Ventajas|Desventajas|
|---|---|---|
|**2-Phase Locking**|Garantiza consistencia estricta y evita que el trabajo realizado se pierda por conflictos de último momento.|Puede generar **deadlocks** y aumentar los tiempos de espera (latencia) mientras una caja espera que otra libere un producto.|
|**Timestamps**|Provee un ordenamiento total y justo basado en la llegada.|En alta contención, genera una **elevada tasa de abortos** si las transacciones se intercalan frecuentemente, obligando a reintentos constantes.|
|**Concurrencia Optimista**|Ofrece el máximo paralelismo y está libre de deadlocks, ideal si no hubiera choques.|**Muy ineficiente bajo alta carga**; el costo de abortar y "rehacer todo" tras detectar un conflicto al final es extremadamente alto en términos de recursos.|

Elección y Justificación

Para esta situación, la opción más recomendable es el **2-Phase Locking (2PL)**.

**Justificación:** Aunque la programación asincrónica y los modelos optimistas son excelentes para la escalabilidad general, en un escenario específico de **mucha contención**, la concurrencia optimista fallaría sistemáticamente, desperdiciando ciclos de CPU en transacciones que terminarán abortadas al intentar el commit. El modelo de **Timestamps** sufriría un problema similar de rollbacks frecuentes.

El **2PL**, a pesar de introducir esperas, asegura que una vez que una caja toma el control de un recurso, la operación llegará a término de forma segura, evitando el costo operativo de procesar ventas que luego deben ser anuladas y reintentadas por conflictos de datos. Para mitigar su principal desventaja (deadlocks), se podrían implementar mecanismos de prevención como **Wound-Wait**, donde las transacciones más viejas pueden forzar el aborto de las más nuevas para evitar bloqueos circulares.

### Comunicación y Redes

#### Qué es un socket y qué servicio utilizaría para implementar una aplicación de streaming de películas. Comparar las características y propiedades de los sockets de Unix con los channels de Rust.

Un **socket** es una herramienta fundamental que permite la comunicación entre procesos, ya sea que se encuentren ejecutándose en la **misma máquina o en máquinas diferentes** a través de una red. Actúan como el punto final de una conexión y son la base del modelo **cliente-servidor**, donde el cliente inicia la interacción y el servidor espera pasivamente las peticiones.

Servicio para una aplicación de streaming

Para implementar una aplicación de **streaming de películas**, el servicio más adecuado es el de **Stream Sockets (TCP)**.

- **Razón:** Los stream sockets utilizan el protocolo TCP, el cual garantiza la **entrega de un flujo de bytes** de manera confiable y ordenada.
- **Propiedad:** En el streaming de películas (especialmente bajo demanda), es vital que no se pierdan datos para evitar artefactos visuales o cortes en la reproducción, por lo que se requiere un **servicio con conexión** que incluya control de flujo y errores.
- Aunque existen los _Datagram sockets_ (UDP), estos no garantizan la entrega, lo que los hace menos ideales para asegurar la calidad de imagen en una película, aunque sean más rápidos.

Comparativa: Sockets de Unix vs. Channels de Rust

| Característica         | Sockets de Unix (IPC/Red)                                                                  | Channels de Rust (`std::sync::mpsc`)                                                                         |
| ---------------------- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| **Alcance**            | Permiten comunicación entre procesos independientes en la **misma o diferentes máquinas**. | Se utilizan para comunicar hilos (**threads**) dentro de un **mismo proceso**.                               |
| **Tipo de Dato**       | Están orientados a **bytes**; la información debe ser serializada (marshall/unmarshall).   | Son **tipados**; permiten enviar estructuras de datos complejas directamente.                                |
| **Gestión de Memoria** | Los procesos tienen **memoria aislada**; los datos se copian a través del kernel.          | Transfieren el **ownership** (propiedad) del dato del emisor al receptor, evitando copias innecesarias.      |
| **Estructura**         | Siguen el modelo **Cliente-Servidor** (pasivo/activo) con llamadas como `bind` y `accept`. | Son de tipo **MPSC** (Múltiples Productores, un solo Consumidor).                                            |
| **Sincronismo**        | Pueden ser bloqueantes o no; el servidor puede ser iterativo o concurrente.                | El receptor (`rx.recv()`) se bloquea automáticamente hasta que llega un mensaje, facilitando el sincronismo. |
| **Identificación**     | Se identifican por **IP y puerto** (Internet) o por una ruta en el **filesystem** (FIFOs). | Se identifican por los extremos del objeto en el código (`tx` para transmitir, `rx` para recibir).           |
En resumen, mientras que los **sockets** son la opción para conectar sistemas distribuidos o procesos aislados mediante el intercambio de bytes, los **channels** de Rust son una abstracción de más alto nivel diseñada para la comunicación segura entre hilos, garantizando que no haya condiciones de carrera mediante el movimiento de la propiedad de los datos. 

#### Describir el modelo OSI y explicar objetivos y capacidades de cada capa. Explicar qué es un socket y qué modelo de concurrencia usan.

El **Modelo OSI** es un marco de referencia que organiza la comunicación en red en **siete capas** jerárquicas. El objetivo principal de esta arquitectura es que **cada capa** N **ofrezca servicios específicos a la capa** N+1, utilizando protocolos particulares para comunicarse con su capa par en otro host.

Las capas que componen este modelo son:

1. **Física**: Define las especificaciones eléctricas y mecánicas del medio de transmisión.
2. **Enlace de Datos**: Provee transferencia de datos nodo a nodo.
3. **Red**: Se ocupa del direccionamiento y envío de paquetes, utilizando protocolos como **IP**.
4. **Transporte**: Capa donde residen protocolos como **TCP** (usado en stream sockets) y **UDP** (usado en datagram sockets) para la transferencia de datos entre extremos.
5. **Sesión**: Gestiona las conexiones entre aplicaciones.
6. **Presentación**: Se encarga del formato de los datos.
7. **Aplicación**: Es la capa superior que interactúa con el usuario final.

Sockets: Definición y Funcionamiento

Un **socket** se define como una herramienta que permite la **comunicación entre procesos**, permitiendo que estos intercambien información independientemente de si residen en la misma máquina o en equipos distintos conectados por una red. Son el componente esencial para el modelo **cliente-servidor**, donde el cliente inicia la interacción (parte activa) y el servidor permanece a la espera de peticiones (parte pasiva).

Existen diversos tipos de sockets según el protocolo de transporte y el nivel de control requerido:

- **Stream sockets**: Basados en **TCP**, garantizan una entrega confiable y ordenada de un flujo de bytes.
- **Datagram sockets**: Basados en **UDP**, operan sin conexión y no garantizan la entrega de los datos.
- **Raw sockets**: Permiten interactuar directamente con paquetes a nivel de **IP**.

**Modelo de Concurrencia de los Sockets**

Desde la perspectiva de los modelos de programación concurrente, los sockets se fundamentan en el modelo de **Pasaje de Mensajes** (Canales y Mensajes).

- **Aislamiento**: A diferencia de los hilos que comparten memoria (Heap), los procesos que se comunican mediante sockets poseen **memoria local no compartida**, lo que elimina por diseño las condiciones de carrera sobre variables globales.
- **Comunicación mediante mensajes**: Las entidades involucradas dividen el problema en partes que intercambian datos mediante el envío de secuencias de bits a través de canales (en este caso, los sockets).
- **Servidores Concurrentes**: Los sockets permiten implementar arquitecturas de **servidor concurrente**, capaces de gestionar múltiples peticiones de clientes de forma simultánea, a diferencia de los servidores iterativos que procesan una sola petición a la vez.
- **Sincronismo**: El receptor de un socket suele operar de forma bloqueante (esperando a que llegue un mensaje) o asincrónica, similar a como funcionan los **canales** en Rust o el modelo de **Actores**, donde el estado solo se modifica al procesar un mensaje entrante.

### Redes de Petri

#### Explicar qué es la función de entrada y salida y el grafo de alcance de una red de Petri y hacer la red de una comunicación cliente-servidor.

En el marco de las **Redes de Petri**, que son grafos bipartitos compuestos por lugares (P) y transiciones (T), el funcionamiento del sistema se rige por la relación entre sus componentes y el flujo de fichas o _tokens_.

Funciones de Entrada, Salida y Grafo de Alcance

De acuerdo con las fuentes, estas funciones definen la estructura y dinámica de la red:

- **Función de Entrada** $I(t)$**:** Para una transición específica $t$, el conjunto de entrada está compuesto por todos los lugares p desde los cuales sale un arco hacia esa transición. En términos lógicos, estos lugares representan las **precondiciones** o datos de entrada necesarios para que el evento (la transición) pueda ocurrir.
- **Función de Salida** $O(t)$**:** El conjunto de salida de una transición $t$ está formado por los lugares $p$ hacia los cuales llega un arco proveniente de dicha transición. Estos lugares representan las **postcondiciones** o los datos de salida una vez que el evento ha finalizado.
- **Grafo de Alcance:** Es un grafo dirigido que representa todos los estados posibles del sistema. Se construye a partir de la **sucesión de funciones de marca** (M) por las que transita la red desde su estado inicial (M0​), mostrando qué configuraciones de _tokens_ son alcanzables mediante el disparo de las transiciones.

---

Red de Petri: Comunicación Cliente-Servidor

Para modelar una comunicación **cliente-servidor** (basada en el modelo donde el cliente inicia la interacción y el servidor espera peticiones), podemos estructurar la red con los siguientes lugares y transiciones:

1. Definición de Lugares (Estados)

- P1​ **(Cliente Ocioso):** Estado inicial del cliente. Tiene un _token_ inicial.
- P2​ **(Petición en Red):** Representa el buffer o canal por donde viaja el mensaje hacia el servidor.
- P3​ **(Cliente Esperando):** El cliente ha enviado la solicitud y espera respuesta.
- P4​ **(Servidor Listo):** El servidor está escuchando (estado pasivo). Tiene un _token_ inicial.
- P5​ **(Servidor Procesando):** El servidor ha recibido la petición y realiza el cómputo.
- P6​ **(Respuesta en Red):** El canal por donde viaja la respuesta hacia el cliente.

2. Definición de Transiciones (Eventos)

- t1​ **(Enviar Petición):** El cliente dispara este evento. Consume el _token_ de P1​ y genera _tokens_ en P2​ (red) y P3​ (espera).
- t2​ **(Recibir Petición):** El servidor toma la petición de la red (P2​) estando listo (P4​).
- t3​ **(Enviar Respuesta):** El servidor termina el proceso (P5​) y coloca la respuesta en la red (P6​), volviendo a estar listo (P4​).
- t4​ **(Recibir Respuesta):** El cliente recibe el mensaje de P6​ estando en espera (P3​) y vuelve a su estado ocioso (P1​).

3. Estructura de las Funciones de Entrada y Salida

Para este modelo, las funciones quedarían definidas de la siguiente manera:

- $I(t1​)={P1​}; O(t1​)={P2​,P3​}$
- $I(t2​)={P2​,P4​}; O(t2​)={P5​}$
- $I(t3​)={P5​}; O(t3​)={P4​,P6​}$
- $I(t4​)={P3​,P6​}; O(t4​)={P1​}$

Este diseño asegura que el servidor sea **iterativo** o **concurrente** dependiendo de la cantidad de _tokens_ iniciales y la estructura de los arcos, y garantiza que el cliente no pueda recibir una respuesta si no envió previamente una petición.

#### Explicar diferencia entre Red de Petri ordinaria vs general. Definición de grafo de alcance y ejemplo. Graficar una red de Petri de productor-consumidor con buffer acotado.

Las **Redes de Petri** son herramientas gráficas y matemáticas para modelar sistemas concurrentes, cuya complejidad varía según la definición de sus componentes y reglas de disparo.

**Diferencia entre Red de Petri Ordinaria y General**

La distinción fundamental radica en la capacidad de los arcos para transportar múltiples fichas (_tokens_) simultáneamente:

- **Red Ordinaria:** Es un grafo bipartito compuesto por un conjunto de **lugares** (P), **transiciones** (T) y **arcos** (A). En este modelo, los arcos tienen una capacidad implícita de **una sola ficha** por disparo; es decir, para que una transición se habilite, basta con que haya al menos una ficha en cada lugar de entrada.
- **Red General:** Introduce una **función de peso** ($W:A→N$) sobre los arcos. Esto significa que un arco puede requerir o producir un número específico (n) de fichas. Una transición solo se habilita si el número de fichas en cada lugar de entrada es mayor o igual al peso del arco correspondiente ($M(p)≥W(p,t)$). Al dispararse, se consumen y producen fichas según el peso definido para cada arco.

**Grafo de Alcance**

El **grafo de alcance** es una representación dirigida que muestra todos los estados posibles que el sistema puede alcanzar desde su configuración inicial. Cada nodo del grafo es una **función de marca** (M) que indica la distribución de fichas en los lugares en un momento dado, y las flechas representan las transiciones que provocan el cambio de un estado a otro.

**Ejemplo:** Imaginemos una red simple con dos lugares ($P1​,P2​$) y una transición ($t1$​) que mueve una ficha de P1​ a P2​:

1. **Estado inicial (M0​)**: Hay 1 ficha en P1​ y 0 en P2​. M0​=(1,0).
2. **Disparo de t1​**: La transición se habilita porque P1​ tiene una ficha. Al dispararse, quita la ficha de P1​ y la pone en P2​.
3. **Nuevo estado (M1​):** M1​=(0,1).
4. **Grafo resultante:** Un nodo M0​ con una flecha etiquetada como t1​ apuntando hacia el nodo M1​.

---

**Red de Petri: Productor-Consumidor con Buffer Acotado**

Para modelar este problema clásico con un buffer de tamaño N, se utilizan lugares que representan tanto los recursos disponibles como los espacios libres para evitar desbordamientos.

Estructura de la Red:

1. **Lugares (P):**
    - $P_{vacio}$​ **(Espacios libres):** Representa el semáforo `notFull`. Se inicializa con N **fichas**, indicando que hay N huecos disponibles en el buffer.
    - $P_{lleno}$​ **(Items disponibles):** Representa el semáforo `notEmpty`. Se inicializa con **0 fichas**, ya que inicialmente el buffer está vacío.
    - $P_{prod}$ :​ Estado del productor listo para trabajar.
    - $P_{cons}​$ : Estado del consumidor listo para recibir.
2. **Transiciones (T):**
    - $t_{producir}$​ : Representa la acción de colocar un elemento en el buffer.
    - $t_{consumir}$ : Representa la acción de retirar un elemento del buffer.

Lógica de los Arcos (Flujo):

- **Para Producir ($t_{producir}$)**:
    - **Entrada:** Debe haber al menos una ficha en $P_{vacio}$​ (hay espacio).
    - **Salida:** Al terminar, se deposita una ficha en $P_{lleno}$​ (hay un item nuevo para consumir).
- **Para Consumir ($t_{consumir}$​)**:
    - **Entrada:** Debe haber al menos una ficha en $P_{lleno}$​ (el buffer no está vacío).
    - **Salida:** Al terminar, se deposita una ficha en $P_{vacio}$​ (se liberó un espacio en el buffer).

Esta red garantiza que el productor se bloquee si el buffer está lleno (cero fichas en $P_{vacio}$​) y que el consumidor espere si el buffer está vacío (cero fichas en $P_{lleno}$​), respetando el **acotamiento del recurso**.