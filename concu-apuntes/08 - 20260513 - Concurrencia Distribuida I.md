## Clase 8 - Concurrencia Distribuida I

### 1. Exclusión Mutua Distribuida

#### El Problema de la Oficina y la Impresora

En una oficina con una impresora compartida, los usuarios suelen enviar documentos simultáneamente. Esto genera inconvenientes porque las impresiones resultan intercaladas en la bandeja de salida, entorpeciendo el flujo de trabajo. Para solucionar este tipo de conflictos de acceso a recursos compartidos en sistemas distribuidos, se utilizan algoritmos de exclusión mutua.

#### Algoritmo Centralizado

En este modelo:

1. Se elige un proceso para que actúe como **coordinador**. 
2. Cuando un proceso desea ingresar a una Sección Crítica (SC), envía una solicitud al coordinador. 
3. Si la SC está libre, el coordinador responde con un mensaje de **OK**; de lo contrario, no responde hasta que la SC se libere.

#### Algoritmo Distribuido (Ricart–Agrawala)

Cuando un proceso quiere entrar en una SC, construye un mensaje que incluye el nombre de la sección, su número de proceso y un **timestamp**. Las reglas al recibir este mensaje son:

1. Si el receptor no está en la SC y no desea entrar, envía un **OK**.
2. Si el receptor ya está en la SC, no responde y encola el mensaje hasta salir, momento en el que envía el **OK**.
3. Si el receptor también desea entrar, se comparan los timestamps y gana el que tenga el valor menor.
4. El proceso solo puede entrar a la SC una vez que ha recibido el **OK de todos** los demás procesos.

#### Algoritmo Token Ring

- Este algoritmo conforma un **anillo mediante conexiones punto a punto**. 
- Al inicio, el proceso 0 recibe un **token** que circula constantemente por el anillo. 
- Solo el poseedor del token tiene permiso para entrar a la SC. 
- Al salir, el token continúa su circulación y el proceso no puede reingresar a otra SC con el mismo token en esa vuelta.

### 2. Modelo Cliente-Servidor y Sockets

#### Introducción a Sockets

Los sockets permiten la comunicación entre procesos, ya sea en la misma máquina o en máquinas diferentes. Son fundamentales para aplicaciones que siguen el modelo cliente-servidor.

- **Cliente:** Es la parte activa, pues inicia la interacción.
- **Servidor:** Es la parte pasiva, ya que espera las peticiones de los clientes. Históricamente, el sistema 4.2BSD (1983) fue el primero en implementar TCP/IP y la API de sockets POSIX. Lenguajes modernos como Rust ofrecen abstracciones de "costo cero" con APIs de sockets sencillas.

#### Arquitectura y Tipos de Servidor

- **Arquitectura de dos niveles:** El cliente se comunica directamente con el servidor.
- **Arquitectura de tres niveles:** Incluye un **middleware** entre el cliente y el servidor para proveer seguridad y balanceo de carga.
- **Servidor Iterativo:** Atiende una petición a la vez.
- **Servidor Concurrente:** Puede gestionar múltiples peticiones simultáneamente.

### 3. Repaso de Redes

#### Modelos de Capas

La comunicación en red se organiza en modelos de capas donde **cada capa $N$ ofrece servicios a la capa $N+1$** utilizando protocolos específicos para interactuar con su par en otro host.

- **Modelo OSI:** Consta de 7 capas (Aplicación, Presentación, Sesión, Transporte, Red, Enlace de datos y Física).
- **Modelo TCP/IP:** Se simplifica en capas de Aplicación, Transporte, Internet y Acceso a Red.

#### Tipos de Servicio

1. **Sin conexión:** Los datos se envían sin control de flujo ni de errores.
2. **Sin conexión con ACK:** El receptor confirma cada dato recibido mediante un acuse de recibo (ACK).
3. **Con conexión:** Incluye fases de establecimiento, intercambio de datos y cierre, garantizando control de flujo y errores.

### 4. Comunicación mediante Sockets

#### Tipos de Sockets

- **Stream sockets:** Utilizan **TCP** para garantizar la entrega de un flujo de bytes.
- **Datagram sockets:** Utilizan **UDP**; la entrega no está garantizada y es un servicio sin conexión.
- **Raw sockets:** Permiten el envío directo de paquetes IP.
- **Sequenced packet sockets:** Similares a los de flujo pero preservan delimitadores de registro (protocolo SPP).

#### Llamadas al Sistema (API)

Para establecer la comunicación se utilizan diversas funciones:

- **socket()**: Crea el descriptor del socket especificando la familia (ej. `AF_INET` para IPv4), el tipo (`SOCK_STREAM` o `SOCK_DGRAM`) y el protocolo. Retorna un entero positivo o -1 en caso de error.
- **connect()**: Utilizada por el cliente para iniciar la conexión con el servidor usando su IP y puerto.
- **bind()**: El servidor asigna una dirección local y un puerto al socket.
- **listen()**: Convierte el socket en pasivo y define el `backlog` (máximo de conexiones pendientes).
- **accept()**: El servidor extrae la siguiente conexión de la cola y retorna un nuevo descriptor para comunicarse específicamente con ese cliente.
- **Lectura/Escritura**: Se usan `read()` y `write()` de forma general, `send()` y `recv()` para streams, y `sendto()` y `recvfrom()` para datagramas.
- **close()**: Finaliza la conexión y cierra el descriptor.

#### Estructuras de Datos

- **struct sockaddr**: Estructura genérica para guardar información de la dirección del socket.
- **struct sockaddr_in**: Estructura específica para IPv4 que incluye la familia (`AF_INET`), el puerto (`sin_port`) y la dirección IP (`sin_addr`).
- **struct in_addr**: Contiene el miembro `s_addr` que representa la dirección IP, frecuentemente usada como `INADDR_ANY` para servidores.

### Algoritmos en Rust

#### 1. Algoritmo Centralizado

En este modelo, un proceso actúa como **coordinador**. Los demás procesos le solicitan permiso para entrar a la Sección Crítica (SC).

```rust
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};

// Lógica simplificada del Coordinador
fn coordinador() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    let mut ocupado = false;
    let mut cola_espera = Vec::new();

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let mut buffer = [0; 10];
        stream.read(&mut buffer).unwrap();
        let mensaje = String::from_utf8_lossy(&buffer);

        if mensaje.starts_with("REQUEST") {
            if !ocupado {
                ocupado = true;
                stream.write_all(b"OK").unwrap(); 
            } else {
                // Si está ocupado, no responde o encola (simplificado)
                cola_espera.push(stream); 
            }
        } else if mensaje.starts_with("RELEASE") {
            ocupado = false;
            if let Some(mut siguiente) = cola_espera.pop() {
                ocupado = true;
                siguiente.write_all(b"OK").unwrap(); 
            }
        }
    }
}
```

#### 2. Algoritmo Distribuido (Ricart–Agrawala)

Este algoritmo no requiere un coordinador central. Los procesos se comunican entre sí enviando mensajes con su **ID y un timestamp**.

```rust
// Estructura lógica del mensaje según las fuentes 
struct Mensaje {
    proceso_id: u32,
    timestamp: u64,
}

fn procesar_solicitud(yo_quiero_entrar: bool, mi_timestamp: u64, msg_recibido: Mensaje) -> &'static str {
    // 1. Si no quiero entrar, envío OK
    // 2. Si ya estoy adentro, encolo (aquí devolvemos "WAIT")
    // 3. Si ambos queremos, gana el timestamp menor 
    
    if !yo_quiero_entrar {
        "OK"
    } else if msg_recibido.timestamp < mi_timestamp {
        "OK" // Gana el que envió el mensaje por tener menor timestamp 
    } else {
        "WAIT" // Encolar el mensaje hasta salir de la SC 
    }
}
```

#### 3. Algoritmo Token Ring

Los procesos forman un **anillo lógico** y un **token** circula entre ellos. Solo el poseedor del token entra a la SC.

```rust
use std::net::{TcpStream};
use std::io::{Read, Write};

fn nodo_anillo(id: u32, siguiente_ip: &str, tiene_token_inicial: bool) {
    let mut tiene_token = tiene_token_inicial;

    loop {
        if tiene_token {
            // Entrar a Sección Crítica si es necesario 
            println!("Proceso {} en SC usando el Token", id);
            
            // Pasar el token al siguiente en el anillo 
            let mut stream = TcpStream::connect(siguiente_ip).expect("Error al conectar al siguiente");
            stream.write_all(b"TOKEN").unwrap();
            tiene_token = false;
        } else {
            // Esperar a recibir el token del nodo anterior
            // (Lógica de servidor para recibir el mensaje "TOKEN")
        }
    }
}
```

