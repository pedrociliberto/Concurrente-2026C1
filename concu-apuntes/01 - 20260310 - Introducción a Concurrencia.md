## Clase 1 - Introducción a Concurrencia

### Definiciones

**Programa**: conjunto de datos, asignaciones e instrucciones de control de flujo que compilan a instrucciones de máquina, las cuales se ejecutan secuencialmente en un procesador y acceden a datos almacenados en memoria principal o secundarias.

**Programa concurrente**: conjunto de programas secuenciales que pueden ejecutarse en paralelo.

**Proceso**: cada uno de los programas secuenciales que conforman el programa concurrente.

**Sistema paralelo**: sistema compuesto por varios programas que se ejecutan simultáneamente en procesadores distintos.

**Multitasking**: ejecución de múltiples procesos concurrentemente en un cierto periodo de tiempo. El *scheduler* se encarga de coordinar el acceso a los procesadores, y forma parte del kernel del sistema operativo.

**Multithreading**: herramienta de algunos lenguajes de programación que permite la ejecución concurrente de threads dentro del mismo programa.

### Desafíos de la concurrencia

Se necesita sincronizar y comunicar procesos diferentes:
- **Sincronización**: coordinación temporal entre distintos procesos
- **Comunicación**: datos que necesitan compartir los procesos para cumplir la función del programa

A partir de estos desafíos y objetivos surgen las siguientes definiciones:

- **Programa concurrente**: conjunto finito de procesos secuenciales.
- **Proceso**: compuesto por un conjunto finito de instrucciones atómicas.
- **Ejecución del programa concurrente**: resulta al ejecutar una secuencia de instrucciones atómicas que se obtiene de intercalarlas arbitrariamente de los procesos que lo componen.
- **Escenario**: una posible ejecución del programa concurrente (el orden en el que se ejecutan las instrucciones de todos los procesos en juego). Los procesos son independientes pero se intercalan las ejecuciones de sus instrucciones.

Una **instrucción atómica** es una instrucción que puede:
- Ejecutarse de principio a fin sin interrupciones; o 
- Directamente no ejecutarse

### Modelos de Concurrencia

- **Estado mutable compartido**: cualquier proceso que comparte las variables globales del programa puede modificarlas. Si se administra erróneamente el acceso a esas variables, puede quedar un estado inconsistente en el programa. 
	- Los procesos se ejecutan al mismo tiempo, pero habra casos en los que solo un procedimiento pueda suceder a la vez.
	- Cualquier otro proceso que intente ejecutar cualquier procedimiento será obligado a esperar hasta que la primera ejecución haya terminado.
	- Se puede serializar para controlar el acceso a las variables compartidas, y marcar regiones del código que no pueden superponerse en la ejecución al mismo tiempo.
- **Paralelismo fork-join**: se acerca al modelo de ejecución en simultáneo de las operaciones. El problema principal debe poder dividirse en subtareas, de tal manera que cada una sea independiente del resto y no necesite datos de ellas. El resultado final sale de unificar (en la etapa de **join**) estas subtareas.
- **Canales y mensajes**: dividir el problema en partes que necesiten datos de otras partes. Para esto, se debe poder comunicar a las partes entre sí para que se manden mensajes. Así es que las partes que conforman el programa se envían *mensajes* a través de *canales*.
- **Programación asincrónica**: el programa está compuesto por tareas sencillas que cooperan entre sí en recursos y CPU. Generalmente se ejecutan en un único procesador, y se suelen utilizar en programas con alto movimiento de entrada de salida (se aprovechan los momentos muertos de espera para darle tiempo de CPU a otras tareas).
- **Actores**: cada actor tiene su estado interno y procesa los datos a través de mensajes con otros actores, y tienen su casilla de envío y recepción de mensajes. Cada actor gestiona su propio estado sin compartir memoria.

### Threads

Los hilos (*threads*) comparten los recursos del proceso, entre ellos, el espacio de memoria. Cada *thread* mantiene su propia información de estado:
- Stack
- PC (*program counter*)
- Registros