//! estacion.rs
//!
//! Módulo principal del proceso de la estación. Contiene la función `main` que inicializa el sistema,
//! configura el actor `Estacion`, y lanza las tareas asíncronas para manejar conexiones TCP, UDP y
//! elecciones de líder.
//!

pub mod actor;
mod config;
pub mod eleccion_de_lider;
pub mod errores_estacion;
pub mod mensajes_internos;

use actix::{Actor, Addr, System, spawn};
use std::{
    env,
    net::SocketAddr,
    thread::{self, JoinHandle},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, WriteHalf},
    join,
    net::{TcpListener, TcpStream, UdpSocket},
    sync::mpsc,
};

use crate::mensajes_internos::{
    CambiarEstadoConectividad, ObtenerLiderActualMsg, SolicitarEstadoMsg,
};
use errores_estacion::EstacionError;
use tp::{
    constantes::{ADDR_BASE, PUERTO_BASE_ESTACION},
    msjs_app_usuario_estacion::{
        Bicicleta, BicicletaDevueltaCorrectamente, DevolverBicicleta, EntregarBicicleta,
        EnviarLiderActual, HayPedidoEnProcesoEnEseSlot, NoSePudoDevolverBicicletaEnSlot,
        NoTengoBicicletaEnEseSlot, ObtenerLiderActual, PagoRechazado, PedirBicicleta,
        SolicitarEstado,
    },
    msjs_app_usuario_estacion_lider::VisualizarEstadoEstaciones,
};

use {
    actor::Estacion,
    eleccion_de_lider::EleccionLider,
    mensajes_internos::{
        ConfigurarRingMsg, DevolverBicicletaMsg, ObtenerVisualizacionEstacionesMsg,
        PedirBicicletaMsg,
    },
};

use crate::config::crear_actor_estacion;

const MONTO_DE_SEGURIDAD: usize = 100; // Monto de seguridad que se cobra al usuario al iniciar un viaje, se devuelve al finalizar el viaje.
const COSTO_POR_SEGUNDO: usize = 1; // Costo por segundo de viaje.

/// Punto de entrada principal para el proceso de la estación.
/// Inicializa la estación leyendo los argumentos de línea de comandos, configura el sistema de actores (Actix)
/// e inicia las tareas asíncronas para escuchar conexiones TCP, UDP, elecciones de líder y eventos de teclado.
///
/// # Retornos
/// - `Result<(), EstacionError>`: Retorna `Ok(())` si el sistema se inicia y ejecuta correctamente, o un `EstacionError` en caso de fallo crítico.
fn main() -> Result<(), EstacionError> {
    let args: Vec<String> = env::args().collect();
    let id = args
        .get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .ok_or(EstacionError::InvalidArgs)?;
    let conectado = args.get(2).map(|s| s == "1").unwrap_or(false);

    println!("\n¡Bienvenido al sistema de estación de bicicletas de la ciudad!");

    // Primero el usuario ve todas las estaciones existentes en el sistema, y las bicicletas que él tiene en uso.

    let sistema_actix = System::new();
    let resultado_inicializacion = sistema_actix.block_on(async {
        let actor = crear_actor_estacion(id, conectado)?;
        let estacion_addr = actor.start();

        let tcp_addr_str = format!("{}:{}", ADDR_BASE, PUERTO_BASE_ESTACION + id as u16);
        let tcp_addr = tcp_addr_str.parse::<SocketAddr>()?;

        let handle = escuchar_cambios_conectividad(estacion_addr.clone());

        let _ = join!(
            escuchar_mensajes_udp(id, estacion_addr.clone()),
            manejar_usuarios_entrantes(tcp_addr, estacion_addr.clone()),
            escuchar_elecciones(id, estacion_addr.clone(), conectado),
        );

        if let Err(e) = handle.join() {
            eprintln!("Error al sincronizar el hilo de conectividad: {:?}", e);
        }
        Ok::<(), EstacionError>(())
    });

    resultado_inicializacion?;

    sistema_actix.run().map_err(EstacionError::IoError)
}

/// Inicializa y configura el sistema de elección de líder (Ring UDP) para la estación.
/// Una vez inicializado, envía un mensaje al actor `Estacion` para registrar el anillo.
///
/// # Parámetros
/// - `id_estacion`: ID numérico único de la estación.
/// - `estacion_addr`: Dirección del actor `Estacion` para enviarle el mensaje de configuración.
/// - `conectado`: Estado inicial de conectividad de la estación.
async fn escuchar_elecciones(id_estacion: usize, estacion_addr: Addr<Estacion>, conectado: bool) {
    match EleccionLider::new(id_estacion, estacion_addr.clone(), conectado).await {
        Ok(ring) => {
            println!(
                "[Estación {}] Ring de elección configurado correctamente.",
                id_estacion
            );
            estacion_addr.do_send(ConfigurarRingMsg(ring));
        }
        Err(e) => {
            eprintln!(
                "[Estación {}] Error crítico al inicializar EleccionLider: {}",
                id_estacion, e
            );
        }
    }
}

/// Inicia un bucle infinito en una tarea separada para escuchar mensajes UDP entrantes.
/// Se encarga de recibir los mensajes de los usuarios y derivarlos a la función de procesamiento.
///
/// # Parámetros
/// - `id`: ID numérico único de la estación, utilizado para calcular el puerto de escucha.
/// - `estacion_addr`: Dirección del actor `Estacion` para consultar el estado interno.
async fn escuchar_mensajes_udp(id: usize, estacion_addr: Addr<Estacion>) {
    spawn(async move {
        let addr_str = format!("{}:{}", ADDR_BASE, PUERTO_BASE_ESTACION + id as u16);
        if let Ok(socket) = UdpSocket::bind(&addr_str).await {
            let mut buf = [0u8; 256];
            loop {
                if let Ok((leido, usuario)) = socket.recv_from(&mut buf).await {
                    procesar_mensaje_udp(&buf[..leido], &socket, usuario, &estacion_addr).await;
                }
            }
        } else {
            eprintln!("[Error UDP] No se pudo enlazar el socket en {}", addr_str);
        }
    });
}

/// Analiza un mensaje UDP entrante, determina su tipo y llama a la función correspondiente
/// para manejar la solicitud (estado, líder o visualización de estaciones).
///
/// # Parámetros
/// - `mensaje`: Slice de bytes que contiene el mensaje serializado recibido.
/// - `socket`: Referencia al socket UDP para poder enviar una respuesta al usuario.
/// - `usuario`: Dirección `SocketAddr` del usuario (remitente) que envió el mensaje.
/// - `estacion_addr`: Dirección del actor `Estacion` para interactuar con su estado interno.
async fn procesar_mensaje_udp(
    mensaje: &[u8],
    socket: &UdpSocket,
    usuario: SocketAddr,
    estacion_addr: &Addr<Estacion>,
) {
    if SolicitarEstado::from_bytes(mensaje).is_ok() {
        if let Err(e) = procesar_solicitud_de_estado(socket, usuario, estacion_addr).await {
            eprintln!("Error procesando solicitud UDP de estado: {:?}", e);
        }
    } else if ObtenerLiderActual::from_bytes(mensaje).is_ok() {
        if let Err(e) = procesar_solicitud_de_lider(socket, usuario, estacion_addr).await {
            eprintln!("Error procesando solicitud UDP de líder: {:?}", e);
        }
    } else if let Ok(respuesta) = VisualizarEstadoEstaciones::from_bytes(mensaje) {
        if let Err(e) =
            procesar_visualizacion_estaciones(socket, usuario, estacion_addr, respuesta).await
        {
            eprintln!("Error procesando solicitud UDP de visualización: {:?}", e);
        }
    } else {
        println!("Mensaje recibido no pudo ser reconocido: {:?}", mensaje);
    }
}

/// Procesa la solicitud de estado general de la estación. Consulta al actor `Estacion`
/// sobre los slots libres y ocupados, y responde al usuario mediante UDP.
///
/// # Parámetros
/// - `socket`: Referencia al socket UDP para enviar la respuesta.
/// - `usuario`: Dirección del remitente al cual enviar el estado.
/// - `estacion_addr`: Dirección del actor `Estacion`.
///
/// # Retornos
/// - `Result<(), EstacionError>`: `Ok(())` si la solicitud se procesó y respondió exitosamente, o un error en caso contrario.
async fn procesar_solicitud_de_estado(
    socket: &UdpSocket,
    usuario: SocketAddr,
    estacion_addr: &Addr<Estacion>,
) -> Result<(), EstacionError> {
    println!("Solicitud de estado recibida.\nUsuario: {:?}", usuario);
    let respuesta = estacion_addr.send(SolicitarEstadoMsg).await?;
    socket.send_to(&respuesta.as_bytes(), usuario).await?;
    Ok(())
}

/// Procesa la solicitud para obtener el líder actual del sistema. Consulta al actor `Estacion`
/// y, si se conoce el líder, envía su ID al usuario por UDP.
///
/// # Parámetros
/// - `socket`: Referencia al socket UDP para enviar la respuesta.
/// - `usuario`: Dirección del remitente al cual enviar la respuesta.
/// - `estacion_addr`: Dirección del actor `Estacion`.
///
/// # Retornos
/// - `Result<(), EstacionError>`: `Ok(())` si la operación se realizó sin errores.
async fn procesar_solicitud_de_lider(
    socket: &UdpSocket,
    usuario: SocketAddr,
    estacion_addr: &Addr<Estacion>,
) -> Result<(), EstacionError> {
    println!("Solicitud de líder recibida.\nUsuario: {:?}", usuario);
    let respuesta = estacion_addr.send(ObtenerLiderActualMsg).await?;
    if let Some(lider) = respuesta {
        let respuesta = EnviarLiderActual(lider);
        socket.send_to(&respuesta.as_bytes(), usuario).await?;
    }
    Ok(())
}

/// Procesa una solicitud de visualización de múltiples estaciones. Le envía un mensaje al
/// actor `Estacion` para recopilar la información de las estaciones solicitadas y devuelve
/// la información formateada al usuario por UDP.
///
/// # Parámetros
/// - `socket`: Referencia al socket UDP para enviar la respuesta.
/// - `usuario`: Dirección del remitente.
/// - `estacion_addr`: Dirección del actor `Estacion`.
/// - `respuesta`: Objeto `VisualizarEstadoEstaciones` recibido previamente y que contiene los IDs solicitados.
///
/// # Retornos
/// - `Result<(), EstacionError>`: `Ok(())` si la operación se realizó exitosamente.
async fn procesar_visualizacion_estaciones(
    socket: &UdpSocket,
    usuario: SocketAddr,
    estacion_addr: &Addr<Estacion>,
    respuesta: VisualizarEstadoEstaciones,
) -> Result<(), EstacionError> {
    println!(
        "Solicitud de visualización de estaciones recibida.\nUsuario: {:?}",
        usuario
    );
    let respuesta = estacion_addr
        .send(ObtenerVisualizacionEstacionesMsg {
            estaciones: respuesta.estaciones,
        })
        .await?;
    socket.send_to(&respuesta.as_bytes(), usuario).await?;
    Ok(())
}

/// Inicia un servidor TCP para escuchar conexiones entrantes de aplicaciones de usuarios.
/// Por cada nueva conexión aceptada, delega su manejo a una nueva tarea asíncrona.
///
/// # Parámetros
/// - `tcp_addr`: Dirección en la cual el servidor TCP escuchará conexiones.
/// - `estacion_addr`: Dirección del actor `Estacion` compartida con cada cliente conectado.
async fn manejar_usuarios_entrantes(tcp_addr: SocketAddr, estacion_addr: Addr<Estacion>) {
    match TcpListener::bind(tcp_addr).await {
        Ok(listener) => loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    println!("Nueva conexión entrante: {:?}", peer_addr);
                    let estacion_addr_clone = estacion_addr.clone();
                    spawn(async move {
                        manejar_usuario_individual(stream, estacion_addr_clone).await;
                    });
                }
                Err(e) => {
                    println!("Error al aceptar conexión TCP: {:?}", e);
                }
            }
        },
        Err(e) => {
            eprintln!(
                "[Error TCP] No se pudo enlazar el servidor TCP en {:?}: {:?}",
                tcp_addr, e
            );
        }
    }
}

/// Maneja el ciclo de vida de una conexión TCP individual con un usuario.
/// Lee el mensaje enviado por el cliente y lo despacha para su procesamiento de comandos.
///
/// # Parámetros
/// - `stream`: Conexión TCP establecida con el cliente.
/// - `estacion_addr`: Dirección del actor `Estacion`.
async fn manejar_usuario_individual(stream: TcpStream, estacion_addr: Addr<Estacion>) {
    let peer = stream.peer_addr().ok();
    let (read_half, writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);

    match recibir_mensaje(&mut reader).await {
        Ok(Some(mensaje)) => {
            println!("Mensaje recibido de {:?}: {:?}", peer, mensaje);
            if let Err(e) = procesar_comando(mensaje, &estacion_addr, writer).await {
                eprintln!("Error procesando comando para {:?}: {:?}", peer, e);
            }
        }
        Ok(None) => {
            println!("Conexión cerrada por el usuario: {:?}", peer);
        }
        Err(e) => {
            eprintln!("Error al leer mensaje de {:?}: {:?}", peer, e);
        }
    }
}

/// Lee asíncronamente un mensaje del flujo de entrada TCP. El mensaje se espera en un formato
/// donde primero se envían 4 bytes indicando la longitud, seguidos del contenido en bytes.
///
/// # Parámetros
/// - `reader`: Lector asíncrono con buffer sobre la mitad de lectura del `TcpStream`.
///
/// # Retornos
/// - `Result<Option<Vec<u8>>, std::io::Error>`:
///   - `Ok(Some(Vec<u8>))` con los bytes leídos del mensaje.
///   - `Ok(None)` si la conexión se cerró inesperadamente (`UnexpectedEof`).
///   - `Err` en caso de un error de lectura de I/O.
async fn recibir_mensaje(
    reader: &mut BufReader<tokio::io::ReadHalf<TcpStream>>,
) -> Result<Option<Vec<u8>>, std::io::Error> {
    let mut len_buf = [0u8; 4];

    if let Err(e) = reader.read_exact(&mut len_buf).await {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            return Ok(None);
        }
        return Err(e);
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buffer = vec![0u8; len];
    reader.read_exact(&mut buffer).await?;

    Ok(Some(buffer))
}

/// Identifica y procesa comandos operativos (como pedir o devolver una bicicleta) provenientes
/// de un cliente a través de TCP, derivando cada uno a su manejador específico.
///
/// # Parámetros
/// - `mensaje`: Vector de bytes del mensaje recibido.
/// - `estacion_addr`: Dirección del actor `Estacion` para procesar el dominio del negocio.
/// - `writer`: Mitad de escritura del flujo TCP para enviar la respuesta.
///
/// # Retornos
/// - `Result<(), EstacionError>`: `Ok(())` si el comando se procesó exitosamente o `EstacionError` en caso de falla.
async fn procesar_comando(
    mensaje: Vec<u8>,
    estacion_addr: &Addr<Estacion>,
    writer: WriteHalf<TcpStream>,
) -> Result<(), EstacionError> {
    if let Ok(pedir_bici) = PedirBicicleta::from_bytes(&mensaje) {
        println!("Procesando comando PedirBicicleta: {:?}", pedir_bici);
        let addr_clone = estacion_addr.clone();
        procesar_pedido_bicicleta(pedir_bici, &addr_clone, writer).await?;
        return Ok(());
    }
    if let Ok(devolver_bici) = DevolverBicicleta::from_bytes(&mensaje) {
        println!("Procesando comando DevolverBicicleta: {:?}", devolver_bici);
        procesar_devolucion_bicicleta(devolver_bici, estacion_addr, writer).await?;
        return Ok(());
    }
    println!("Comando no reconocido: {:?}", mensaje);
    Ok(())
}

/// Maneja la lógica específica para la devolución de una bicicleta por parte de un usuario.
/// Se comunica con el actor `Estacion` y notifica al usuario si fue exitoso y el monto cobrado,
/// o si hubo algún error (e.g. slot ocupado).
///
/// # Parámetros
/// - `devolver_bici`: Estructura deserializada con los datos de la devolución.
/// - `estacion_addr`: Dirección del actor `Estacion`.
/// - `writer`: Flujo de escritura TCP hacia el cliente para notificar el resultado.
///
/// # Retornos
/// - `Result<(), EstacionError>`: `Ok(())` tras finalizar el flujo de envío o un error si ocurre un problema de comunicación/red.
async fn procesar_devolucion_bicicleta(
    devolver_bici: DevolverBicicleta,
    estacion_addr: &Addr<Estacion>,
    mut writer: WriteHalf<TcpStream>,
) -> Result<(), EstacionError> {
    let num_slot = devolver_bici.numero_slot;

    let monto_cobro = estacion_addr
        .send(DevolverBicicletaMsg(devolver_bici))
        .await?;

    let bytes_a_enviar = if let Some(m) = monto_cobro {
        println!("Devolución procesada, monto a cobrar al usuario: {}", m);
        BicicletaDevueltaCorrectamente.as_bytes()
    } else {
        NoSePudoDevolverBicicletaEnSlot {
            numero_slot: num_slot,
        }
        .as_bytes()
    };

    let len = bytes_a_enviar.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&bytes_a_enviar).await?;
    writer.flush().await?;
    println!("Respuesta enviada al usuario con éxito.");
    Ok(())
}

/// Maneja la lógica específica para la petición de retiro de una bicicleta.
/// Delega el pedido al actor `Estacion` y responde al cliente según la disponibilidad,
/// si hay pagos en proceso, o si se entregó correctamente.
///
/// # Parámetros
/// - `pedir_bici`: Estructura deserializada con los datos del pedido (usuario, slot, tarjeta).
/// - `estacion_addr`: Dirección del actor `Estacion`.
/// - `writer`: Flujo de escritura TCP hacia el cliente.
///
/// # Retornos
/// - `Result<(), EstacionError>`: `Ok(())` en caso de éxito o propagación de un `EstacionError`.
async fn procesar_pedido_bicicleta(
    pedir_bici: PedirBicicleta,
    estacion_addr: &Addr<Estacion>,
    mut writer: WriteHalf<TcpStream>,
) -> Result<(), EstacionError> {
    let (tx, mut rx) = mpsc::channel(128);
    let num_slot = pedir_bici.numero_slot;
    let respuesta = estacion_addr
        .send(PedirBicicletaMsg(pedir_bici, tx))
        .await?;
    // option<bool> -> Some(true) -> todo ok, Some(false): pendiente, None -> no tengo bici

    let bytes_a_enviar = match respuesta {
        Some(true) => {
            let respuesta_canal = rx.recv().await.ok_or(EstacionError::CanalCerrado)?;
            if let Ok(bicicleta) = Bicicleta::from_bytes(&respuesta_canal) {
                EntregarBicicleta { bicicleta }.as_bytes()
            } else {
                PagoRechazado.as_bytes()
            }
        }
        Some(false) => HayPedidoEnProcesoEnEseSlot {
            numero_slot: num_slot,
        }
        .as_bytes(),
        None => NoTengoBicicletaEnEseSlot {
            numero_slot: num_slot,
        }
        .as_bytes(),
    };

    let len = bytes_a_enviar.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&bytes_a_enviar).await?;
    writer.flush().await?;
    println!("Respuesta enviada al usuario con éxito.");
    Ok(())
}

/// Ejecuta un hilo de fondo que bloquea esperando un salto de línea (Enter) por la consola.
/// Al recibirlo, envía un mensaje al actor `Estacion` para alternar el estado de conectividad simulado.
///
/// # Parámetros
/// - `estacion_addr`: Dirección del actor `Estacion`.
///
/// # Retornos
/// - `JoinHandle<()>`: El manejador del hilo creado, permitiendo sincronizar su finalización.
fn escuchar_cambios_conectividad(estacion_addr: Addr<Estacion>) -> JoinHandle<()> {
    thread::spawn(move || {
        println!("Escuchando cambios de conectividad...");
        loop {
            if esperar_input().is_ok() {
                estacion_addr.do_send(CambiarEstadoConectividad);
            } else {
                eprintln!("[Error] Falló la lectura de stdin en el hilo de conectividad.");
                break;
            }
        }
    })
}

/// Espera bloqueando la ejecución hasta que el usuario ingrese un salto de línea por la entrada estándar (stdin).
///
/// # Retornos
/// - `Result<(), std::io::Error>`: `Ok(())` tras leer la línea exitosamente o un error de I/O si falla la lectura.
fn esperar_input() -> Result<(), std::io::Error> {
    std::io::stdin().read_line(&mut String::new())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::Estacion;
    use actix::Actor;
    use std::collections::{HashMap, HashSet, VecDeque};
    use tokio::net::{TcpListener, TcpStream, UdpSocket};
    use tp::coordenadas::Coordenadas;
    use tp::msjs_app_usuario_estacion::{EnviarEstado, EnviarLiderActual};
    use tp::msjs_app_usuario_estacion_lider::{EstacionesPedidas, VisualizarEstadoEstaciones};

    // Función constructora auxiliar para crear un Actor Estacion controlado
    fn crear_estacion_dummy() -> Estacion {
        Estacion {
            id: 1,
            nombre: "Estacion Prueba".to_string(),
            slots: vec![],
            coordenadas: Coordenadas::new(0, 0),
            conectado: true,
            tx_tcp: None,
            otras_estaciones: HashSet::new(),
            lider_actual: Some(1),
            procesador_de_pagos: "127.0.0.1:8080".parse().unwrap(),
            estaciones_info: vec![],
            ring_eleccion: None,
            servidor_tcp_iniciado: false,
            seguidores_tx: HashMap::new(),
            alquileres_activos: HashMap::new(),
            pagos_pendientes: VecDeque::new(),
        }
    }

    #[tokio::test]
    async fn test01_recibir_mensaje_exitoso() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let mut cliente = TcpStream::connect(addr).await.unwrap();
        let (servidor, _) = listener.accept().await.unwrap();

        let payload = b"hola mundo";
        let len = (payload.len() as u32).to_be_bytes();

        cliente.write_all(&len).await.unwrap();
        cliente.write_all(payload).await.unwrap();

        let (read_half, _) = tokio::io::split(servidor);
        let mut reader = BufReader::new(read_half);

        let resultado = recibir_mensaje(&mut reader).await.unwrap();
        assert!(resultado.is_some(), "Debería haberse leído el mensaje");
        assert_eq!(resultado.unwrap(), payload);
    }

    #[tokio::test]
    async fn test02_recibir_mensaje_eof_inesperado() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let mut cliente = TcpStream::connect(addr).await.unwrap();
        let (servidor, _) = listener.accept().await.unwrap();

        // Escribimos solo 2 bytes de longitud y cerramos la conexión a la mitad
        cliente.write_all(&[0, 0]).await.unwrap();
        drop(cliente);

        let (read_half, _) = tokio::io::split(servidor);
        let mut reader = BufReader::new(read_half);

        let resultado = recibir_mensaje(&mut reader).await.unwrap();
        assert!(
            resultado.is_none(),
            "Debería retornar None indicando que se cortó la conexión (EOF)"
        );
    }

    #[actix::test]
    async fn test03_procesar_comando_desconocido_no_falla() {
        let estacion = crear_estacion_dummy();
        let addr_estacion = estacion.start();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let _cliente = TcpStream::connect(addr).await.unwrap();
        let (servidor, _) = listener.accept().await.unwrap();
        let (_, writer) = tokio::io::split(servidor);

        let mensaje_invalido = vec![99, 99, 99]; // Byte inválido para cualquier comando

        let resultado = procesar_comando(mensaje_invalido, &addr_estacion, writer).await;
        assert!(
            resultado.is_ok(),
            "El sistema debería ignorarlo y retornar Ok sin pánico"
        );
    }

    #[actix::test]
    async fn test04_procesar_mensaje_udp_basura_no_falla() {
        let estacion = crear_estacion_dummy();
        let addr_estacion = estacion.start();

        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let peer = "127.0.0.1:8080".parse().unwrap();

        let mensaje_basura = b"ESTO NO ES UN MENSAJE VALIDO";

        // Únicamente comprobamos que lo reciba, falle el parseo y retorne normalmente
        procesar_mensaje_udp(mensaje_basura, &socket, peer, &addr_estacion).await;
    }

    #[actix::test]
    async fn test05_procesar_solicitud_de_estado_udp() {
        let estacion = crear_estacion_dummy();
        let addr_estacion = estacion.start();

        let socket_servidor = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let socket_cliente = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr_cliente = socket_cliente.local_addr().unwrap();

        let resultado =
            procesar_solicitud_de_estado(&socket_servidor, addr_cliente, &addr_estacion).await;
        assert!(resultado.is_ok());

        let mut buf = [0u8; 1024];
        let (len, _) = socket_cliente.recv_from(&mut buf).await.unwrap();

        let respuesta = EnviarEstado::from_bytes(&buf[..len]);
        assert!(
            respuesta.is_ok(),
            "La respuesta debe ser una estructura EnviarEstado válida"
        );
    }

    #[actix::test]
    async fn test06_procesar_solicitud_de_lider_udp() {
        let estacion = crear_estacion_dummy(); // Tiene lider 1 en su struct base
        let addr_estacion = estacion.start();

        let socket_servidor = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let socket_cliente = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr_cliente = socket_cliente.local_addr().unwrap();

        let resultado =
            procesar_solicitud_de_lider(&socket_servidor, addr_cliente, &addr_estacion).await;
        assert!(resultado.is_ok());

        let mut buf = [0u8; 1024];
        let (len, _) = socket_cliente.recv_from(&mut buf).await.unwrap();

        let respuesta = EnviarLiderActual::from_bytes(&buf[..len]);
        assert!(respuesta.is_ok());
        assert_eq!(
            respuesta.unwrap().0,
            1,
            "Debería retornar 1 como líder de la red"
        );
    }

    #[actix::test]
    async fn test07_procesar_visualizacion_estaciones_udp() {
        let estacion = crear_estacion_dummy();
        let addr_estacion = estacion.start();

        let socket_servidor = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let socket_cliente = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr_cliente = socket_cliente.local_addr().unwrap();

        let peticion = VisualizarEstadoEstaciones {
            estaciones: vec![1],
        };

        let resultado = procesar_visualizacion_estaciones(
            &socket_servidor,
            addr_cliente,
            &addr_estacion,
            peticion,
        )
        .await;
        assert!(resultado.is_ok());

        let mut buf = [0u8; 1024];
        let (len, _) = socket_cliente.recv_from(&mut buf).await.unwrap();

        let respuesta = EstacionesPedidas::from_bytes(&buf[..len]);
        assert!(
            respuesta.is_ok(),
            "La respuesta debe ser una estructura EstacionesPedidas válida"
        );
    }
}
