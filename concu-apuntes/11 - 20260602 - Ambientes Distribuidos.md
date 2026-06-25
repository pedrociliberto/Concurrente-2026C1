## Clase 11 - Ambientes Distribuidos

### Entidades

- Unidad de cómputo de ambiente informático distribuido 
- Puede ser un proceso, un procesador, etc (a más alto nivel, un Actor)

#### Capacidades

Cada entidad cuenta con las siguientes capacidades:

1. Acceso de lectura y escritura a una memoria local (no compartida con otras entidades): 
	- Registro de estado: **status(x)** 
	- Registro de valor de entrada: **value(x)** 
2. Procesamiento local 
3. Comunicación: preparación, transmisión y recepción de mensajes 
4. Setear y resetear un reloj local

#### Eventos Externos

- La entidad solamente responde a eventos externos (es ***reactiva***). 
- Los posibles **eventos externos** son: 
	- Llegada de un mensaje 
	- Activación del reloj 
	- Un impulso espontáneo
- A excepción del impulso espontáneo, los eventos se generan **dentro de los límites del sistema**.

### Reglas y Comportamientos

#### Acciones y Reglas

**Acción** 
- Secuencia finita e indivisible de operaciones. 
- Es atómica porque se ejecuta sin interrupciones. 

**Regla** 
- Relación entre el evento que ocurre y el estado en el que se encuentra la entidad cuando ocurre dicho evento
- estado × evento → acción
#### Comportamiento

Es el conjunto $B(x)$ de todas las reglas que obedece una entidad $x$:

- Para cada posible evento y estado debe existir una única regla $B(x)$
- $B(x)$ se llama también protocolo o algoritmo distribuido de $x$ 

Comportamiento colectivo del ambiente distribuido: 
$$B(E) = B(x) : ∀x ∈ E$$
#### Comportamiento homogéneo

El comportamiento colectivo es **homogéneo** si todas las entidades que lo componen tienen el mismo comportamiento, o sea: $$∀x, y ∈ E, B(x) = B(y)$$
**Propiedad** 
- Todo comportamiento colectivo se puede transformar en homogéneo.

#### Comunicación

- Una entidad se comunica con otras entidades mediante mensajes (un mensaje es una secuencia finita de bits) 
- Puede ocurrir que una entidad sólo pueda comunicarse con un subconjunto del resto de las entidades: 
	- $N_{OUT} (x) ⊆ E$: conjunto de entidades **a las cuales x puede enviarles un mensaje** directamente.
	- $N_{IN}(x) ⊆ E$: conjunto de entidades **de las cuales x puede recibir un mensaje** directamente.

#### Axiomas

**Delays de comunicación finitos**: En ausencia de fallas los delays en la comunicación tienen una duración finita.

**Orientación local**: Una entidad puede distinguir entre sus vecinos $N_{OUT}$ y entre sus vecinos $N_{IN}$.

- Una entidad puede distinguir **qué vecino** le envía un mensaje.
- Una entidad puede enviar un mensaje a un **vecino específico**.

#### Restricciones de confiabilidad

- **Entrega garantizada**: cualquier mensaje enviado será recibido con su contenido intacto
- **Confiabilidad parcial**: no ocurrirán fallas en el futuro
- **Confiabilidad total**: no han ocurrido ni ocurrirán fallas en el futuro

#### Restricciones temporales

- **Delays de comunicación acotados**: existe una constante $∆$ tal que en ausencia de fallas el delay de cualquier mensaje en el enlace es a lo sumo $∆$.
- **Delays de comunicación unitarios**: en ausencia de fallas, el delay de cualquier mensaje en un enlace es igual a una unidad de tiempo.
- **Relojes sincronizados**: todos lo relojes locales se incrementan simultáneamente y el intervalo de incremento es constante.

### Costo y Complejidad

Medidas de comparaciób de los algoritmos distribuidos.

- **Cantidad** de actividades de comunicación 
	- Cantidad de transmisiones o costo de mensajes ($M$)
	- Carga de trabajo por entidad y carga de transmisión 
- **Tiempo** 
	- **Tiempo total** de ejecución del protocolo
	- **Tiempo ideal** de ejecución: tiempo medido bajo ciertas condiciones, como delays de comunicación unitarios y relojes sincronizados

### Tiempo y Eventos

**Tipos de eventos**:
- Impulso espontáneo
- Recepción de un mensaje
- Alarma del reloj activada

Los eventos desencadenan acciones en un **tiempo futuro**. Los distintos delays resultan en distintas ejecuciones del protocolo con posibles resultados diferentes.

- Los eventos disparan acciones que pueden generar nuevos eventos
- Si suceden, los nuevos eventos ocurrirán en un tiempo futuro: $Future(t)$
- Una ejecución se describe por la secuencia de eventos que ocurrieron

#### Estados y Configuraciones

- Estado interno de $x$ en el instante $t$ ($σ(x,t)$): contenido de los registros de $x$ y el valor del reloj $c_x$ en el instante $t$
- El estado interno de una entidad cambia con la ocurrencia de eventos 

Sea una entidad $x$ que recibe el mismo evento en dos ejecuciones distintas, y $σ_1$ y $σ_2$ los estados internos. Si $σ_1 = σ_2$ ⇒ el nuevo estado interno de $x$ será el **mismo en ambas ejecuciones**.

#### Conocimiento

**Conocimiento local**: 
- Contenido de la memoria local de $x$ y la información que se deriva.
- En ausencia de fallas, el conocimiento no puede perderse.

**Tipos de conocimiento**

- **Información métrica**: información numérica sobre la red. 
	- Ej: número de nodos del grafo ($n = ||V||$), número de arcos del grafo ($m = ||E||$), diámetro del grafo, etc.
- **Propiedades topológicas**: conocimiento sobre propiedades de la topología. 
	- Ej: el grafo es un anillo, el grafo es acíclico, etc.
- **Mapas topológicos**: un mapa de la vecindad de la entidad hasta una distancia $d$. 
	- Ej: matriz de adyacencia del grafo.