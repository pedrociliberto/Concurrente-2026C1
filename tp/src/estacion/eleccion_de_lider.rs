//! eleccion_de_lider.rs
//!
//! Módulo encargado de implementar la elección de líder entre las estaciones utilizando un algoritmo
//! de anillo.
//!

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::{net::SocketAddr, sync::Arc};

use crate::mensajes_internos::{
    CambiarLiderMsg, LiderCaidoMsg, MensajeEntranteTcpMsg, NuevaConexionTcpMsg,
    RegistrarSeguidorMsg, SeguidorCaidoMsg,
};
use actix::Addr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, mpsc};
use tokio::time::timeout;
use tokio::{net::UdpSocket, spawn};
use tp::constantes::{
    ADDR_BASE, CANTIDAD_ESTACIONES, PUERTO_BASE_ELECCION, PUERTO_BASE_SINCRO_TCP, TIMEOUT_UDP,
};

use crate::actor::Estacion;
use crate::errores_estacion::EstacionError;

/// Estructura encargada de manejar el protocolo de elección de líder a través de un anillo lógico mediante UDP.
///
/// # Atributos
/// - `id`: ID numérico único de la estación local.
/// - `socket`: Socket UDP compartido de forma segura (`Arc`) para enviar y recibir mensajes del anillo.
/// - `estacion_addr`: Dirección del actor `Estacion` para notificarle sobre cambios en la red (ej. nuevo líder).
/// - `ack_notify`: Notificador asíncrono para coordinar la recepción de acuses de recibo (ACKs).
/// - `ultimo_lider_coordinado`: Variable atómica que guarda el ID del último líder acordado por el anillo.
/// - `conectado`: Estado atómico que simula la conectividad de la estación (falso = desconectado/caído).
#[derive(Clone)]
pub struct EleccionLider {
    id: usize,
    socket: Arc<UdpSocket>,
    estacion_addr: Addr<Estacion>,
    ack_notify: Arc<Notify>,
    ultimo_lider_coordinado: Arc<AtomicUsize>,
    conectado: Arc<AtomicBool>,
}

impl EleccionLider {
    /// Inicializa una nueva instancia del manejador de elecciones de líder.
    /// Levanta un socket UDP en el puerto correspondiente y lanza las tareas de escucha e inicio de elección.
    ///
    /// # Parámetros
    /// - `id`: ID numérico de la estación base.
    /// - `estacion_addr`: Dirección del actor `Estacion`.
    /// - `conectado_inicial`: Estado de conectividad de red con el que arranca la estación.
    ///
    /// # Retornos
    /// - `Result<Self, EstacionError>`: Retorna la instancia envuelta en un `Ok` si el socket se enlaza con éxito, o un error de red/IO en su defecto.
    pub async fn new(
        id: usize,
        estacion_addr: Addr<Estacion>,
        conectado_inicial: bool,
    ) -> Result<Self, EstacionError> {
        let addr_str = format!("{}:{}", ADDR_BASE, PUERTO_BASE_ELECCION + id as u16);
        let addr = addr_str
            .parse::<SocketAddr>()
            .map_err(|_| EstacionError::InvalidAddress(addr_str))?;

        let socket = UdpSocket::bind(addr).await.map_err(|e| {
            EstacionError::NetworkError(format!("No se pudo enlazar el socket UDP: {}", e))
        })?;

        let eleccion = EleccionLider {
            id,
            socket: Arc::new(socket),
            estacion_addr,
            ack_notify: Arc::new(Notify::new()),
            ultimo_lider_coordinado: Arc::new(AtomicUsize::new(0)),
            conectado: Arc::new(AtomicBool::new(conectado_inicial)),
        };

        let eleccion_clone1 = eleccion.clone();
        spawn(async move {
            eleccion_clone1.escuchar().await;
        });
        let eleccion_clone2 = eleccion.clone();
        spawn(async move {
            eleccion_clone2.iniciar().await;
        });

        Ok(eleccion)
    }

    /// Lanza una tarea asíncrona que inicia el proceso de votación en el anillo.
    /// Utilizado típicamente cuando se detecta de forma pasiva o activa la caída del líder actual.
    pub fn disparar_eleccion_sincrono(&self) {
        let self_clone = self.clone();
        spawn(async move {
            self_clone.iniciar().await;
        });
    }

    /// Resetea el último líder conocido a 0.
    /// Utilizado al momento de reconectarse a la red para forzar al nodo a participar activamente en un nuevo consenso.
    pub fn eliminar_utlimo_lider_coordinado(&self) {
        self.ultimo_lider_coordinado.store(0, Ordering::SeqCst);
    }

    /// Actualiza el estado de conectividad lógica del sistema.
    ///
    /// # Parámetros
    /// - `estado`: Nuevo valor booleano (true = conectado, false = simulando caída).
    pub fn cambiar_estado_de_conectividad(&self, estado: bool) {
        self.conectado.store(estado, Ordering::SeqCst);
    }

    /// Bucle principal infinito que escucha los paquetes UDP entrantes en el puerto del anillo.
    /// Procesa mensajes de acuse (`ACK`), de votación (`ELEC:`) y de coordinación (`COOR:`),
    /// aplicando las reglas del algoritmo de anillo para avanzar los mensajes o declarar un nuevo líder.
    pub async fn escuchar(&self) {
        let mut buf = [0u8; 512];

        loop {
            if let Ok((len, from)) = self.socket.recv_from(&mut buf).await {
                if !self.conectado.load(Ordering::SeqCst) {
                    continue;
                }

                let msg = String::from_utf8_lossy(&buf[..len]).trim().to_string();

                if msg == "ACK" {
                    self.ack_notify.notify_one();
                    continue;
                }

                if msg.starts_with("ELEC:") || msg.starts_with("COOR:") {
                    let _ = self.socket.send_to(b"ACK", from).await;
                }

                if let Some(restante) = msg.strip_prefix("ELEC:") {
                    let mut ids: Vec<usize> =
                        restante.split(',').filter_map(|s| s.parse().ok()).collect();

                    if ids.contains(&self.id) {
                        let nuevo_lider = *ids.iter().max().unwrap_or(&self.id);
                        let ultimo = self.ultimo_lider_coordinado.load(Ordering::SeqCst);
                        if ultimo == nuevo_lider {
                            continue;
                        }
                        println!(
                            "[Anillo {}] ¡Vuelta completada! Nodos activos: {:?}. Líder: {}",
                            self.id, ids, nuevo_lider
                        );
                        self.ultimo_lider_coordinado
                            .store(nuevo_lider, Ordering::SeqCst);
                        self.iniciar_conexion_nuevo_lider(nuevo_lider);
                        let coor_msg = format!("COOR:{}", nuevo_lider);
                        self.enviar_con_reintentos(coor_msg);
                    } else {
                        println!(
                            "[Anillo {}] Recibí ELEC, agregándome y reenviando...",
                            self.id
                        );
                        ids.push(self.id);
                        let nueva_lista = ids
                            .iter()
                            .map(|id| id.to_string())
                            .collect::<Vec<String>>()
                            .join(",");
                        let nuevo_msg = format!("ELEC:{}", nueva_lista);

                        self.enviar_con_reintentos(nuevo_msg);
                    }
                } else if msg.starts_with("COOR:")
                    && let Ok(lider_id) = msg[5..].parse::<usize>()
                {
                    let ultimo = self.ultimo_lider_coordinado.load(Ordering::SeqCst);
                    if ultimo != lider_id {
                        self.ultimo_lider_coordinado
                            .store(lider_id, Ordering::SeqCst);
                        self.iniciar_conexion_nuevo_lider(lider_id);
                        println!(
                            "[Anillo {}] Coordinación recibida para líder {}. Reenviando...",
                            self.id, lider_id
                        );

                        self.enviar_con_reintentos(msg);
                    }
                }
            }
        }
    }

    /// Dispara el mensaje inicial de elección hacia el siguiente nodo del anillo.
    /// Se inyecta a sí mismo (su `id`) en la lista de candidatos con el formato `ELEC:id`.
    pub async fn iniciar(&self) {
        if !self.conectado.load(Ordering::SeqCst) {
            return;
        }

        println!("[Anillo {}] Disparando elección inicial...", self.id);
        let msg_inicial = format!("ELEC:{}", self.id);
        self.enviar_con_reintentos(msg_inicial);
    }

    /// Envía un mensaje UDP a su sucesor lógico en el anillo y aguarda un acuse de recibo (`ACK`).
    /// Si ocurre un timeout (nodo ignorado/caído), avanza al siguiente nodo disponible de forma circular.
    ///
    /// # Parámetros
    /// - `msg`: Cadena de texto a enviar (puede ser de votación `ELEC:` o de coordinación `COOR:`).
    fn enviar_con_reintentos(&self, msg: String) {
        let socket = self.socket.clone();
        let ack_notify = self.ack_notify.clone();
        let id_origen = self.id;
        let estacion_addr = self.estacion_addr.clone();
        // Se lanza en una tarea separada para no bloquear la recepción de mensajes en escuchar()
        spawn(async move {
            let mut id_destino = (id_origen % CANTIDAD_ESTACIONES) + 1;
            let msg_tipo_coor = msg.starts_with("COOR:");
            loop {
                if id_destino == id_origen {
                    if !msg_tipo_coor {
                        println!(
                            "[Anillo {}] Soy el único nodo activo detectado en esta iteración.",
                            id_origen
                        );
                        estacion_addr.do_send(CambiarLiderMsg(id_origen));
                    }
                    break;
                }

                let addr_destino =
                    format!("{}:{}", ADDR_BASE, PUERTO_BASE_ELECCION + id_destino as u16);
                let _ = socket.send_to(msg.as_bytes(), &addr_destino).await;

                match timeout(TIMEOUT_UDP, ack_notify.notified()).await {
                    Ok(_) => {
                        println!(
                            "[Anillo {}] ACK recibido por parte de {}.",
                            id_origen, id_destino
                        );
                        break;
                    }
                    Err(_) => {
                        println!(
                            "[Anillo {}] Nodo {} desconectado (timeout). Saltando al siguiente...",
                            id_origen, id_destino
                        );
                        id_destino = (id_destino % CANTIDAD_ESTACIONES) + 1;
                    }
                }
            }
        });
    }

    /// Notifica al actor de negocio `Estacion` que el algoritmo de anillo ha resuelto el nuevo líder de la red.
    ///
    /// # Parámetros
    /// - `nuevo_lider`: El ID numérico de la estación elegida como coordinador central.
    fn iniciar_conexion_nuevo_lider(&self, nuevo_lider: usize) {
        self.estacion_addr.do_send(CambiarLiderMsg(nuevo_lider));
    }
}

/// Levanta el servidor TCP de una estación una vez que se ha convertido en líder (coordinador).
/// Espera de forma asíncrona las conexiones entrantes de las demás estaciones para su sincronización.
///
/// # Parámetros
/// - `id`: ID numérico del nodo líder, usado para calcular el puerto de escucha.
/// - `estacion_addr`: Dirección local del actor `Estacion`.
pub async fn iniciar_servidor_tcp_lider(id: usize, estacion_addr: Addr<Estacion>) {
    let addr = format!("{}:{}", ADDR_BASE, PUERTO_BASE_SINCRO_TCP + id as u16);
    println!(
        "[Líder TCP {}] Intentando abrir servidor de sincronización en {}",
        id, addr
    );
    match TcpListener::bind(&addr).await {
        Ok(listener) => {
            println!(
                "[Líder TCP {}] Escuchando conexiones de seguidores en {}",
                id, addr
            );
            loop {
                if let Ok((stream, peer)) = listener.accept().await {
                    println!("[Líder TCP {}] Nueva conexión de seguidor: {}", id, peer);
                    manejar_conexion_tcp(stream, estacion_addr.clone(), true).await;
                }
            }
        }
        Err(e) => {
            eprintln!(
                "[Líder TCP {}] ERROR CRÍTICO: No se pudo bindeas el servidor TCP en {}: {}",
                id, addr, e
            );
        }
    }
}

/// Inicia una conexión cliente TCP desde un nodo seguidor hacia el puerto de escucha del nodo coordinador.
/// Efectúa un saludo inicial (`HANDSHAKE`) indicando su ID al conectarse con éxito.
///
/// # Parámetros
/// - `id`: ID numérico propio (seguidor).
/// - `lider_id`: ID numérico del líder con el cual desea establecer la conexión TCP.
/// - `estacion_addr`: Dirección local del actor `Estacion`.
pub async fn conectar_con_lider_tcp(id: usize, lider_id: usize, estacion_addr: Addr<Estacion>) {
    let addr = format!("{}:{}", ADDR_BASE, PUERTO_BASE_SINCRO_TCP + lider_id as u16);
    println!("[Seguidor TCP] Intentando conectar al líder en {}", addr);
    if let Ok(mut stream) = TcpStream::connect(&addr).await {
        println!("[Seguidor TCP] Conectado exitosamente al líder en {}", addr);

        // Mini "handshake" para que el líder conozca el id de la estación "segudiora":
        let handshake = format!("HANDSHAKE:{}\n", id);
        if let Err(e) = stream.write_all(handshake.as_bytes()).await {
            eprintln!("[Seguidor TCP] Error al enviar handshake: {}", e);
            return;
        }

        manejar_conexion_tcp(stream, estacion_addr, false).await;
    } else {
        println!("[Seguidor TCP] Falló la conexión al líder.");
    }
}

/// Gestor del ciclo de vida asíncrono de un socket TCP (válido tanto en modo cliente como en servidor).
/// Configura canales asíncronos para comunicar bidireccionalmente el socket de red con el actor `Estacion`.
///
/// # Parámetros
/// - `stream`: El socket TCP a gestionar.
/// - `estacion_addr`: Dirección del actor `Estacion` para inyectarle de los mensajes de red recibidos.
/// - `es_lider`: Flag para bifurcar las reglas lógicas (e.g. aceptar el `HANDSHAKE` o no).
async fn manejar_conexion_tcp(stream: TcpStream, estacion_addr: Addr<Estacion>, es_lider: bool) {
    let (read_half, mut write_half) = tokio::io::split(stream);
    let (tx, mut rx) = mpsc::channel::<String>(32);

    let tx_para_conexion = tx.clone();
    estacion_addr.do_send(NuevaConexionTcpMsg(tx_para_conexion));

    let tx_para_tarea2 = tx.clone();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if msg.starts_with("DESCONECTAR") {
                if let Err(e) = write_half.shutdown().await {
                    eprintln!(
                        "[TCP Sincro] Error al apagar el canal de escritura de forma ordenada: {:?}",
                        e
                    );
                }
                break;
            }

            let msg_con_salto = format!("{}\n", msg);
            if write_half
                .write_all(msg_con_salto.as_bytes())
                .await
                .is_err()
                || msg.starts_with("DESCONECTAR")
            {
                break;
            }
        }
    });

    // Tarea 2: Escuchar el socket TCP y enviarlo al actor
    tokio::spawn(async move {
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        let mut id_seguidor: Option<usize> = None;

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    let msg_limpio = line.trim().to_string();

                    if es_lider && id_seguidor.is_none() && msg_limpio.starts_with("HANDSHAKE:") {
                        if let Ok(id) = msg_limpio[10..].parse::<usize>() {
                            id_seguidor = Some(id);
                            println!(
                                "[Líder TCP] Handshake recibido: Seguidor {} registrado.",
                                id
                            );

                            let tx_para_seguidor = tx_para_tarea2.clone();
                            estacion_addr.do_send(RegistrarSeguidorMsg {
                                id_seguidor: id,
                                tx: tx_para_seguidor,
                            });
                        }
                        continue;
                    }

                    estacion_addr.do_send(MensajeEntranteTcpMsg(msg_limpio));
                }
                Err(_) => break,
            }
        }
        println!("[TCP] Conexión cerrada.");

        if !es_lider {
            estacion_addr.do_send(LiderCaidoMsg);
        } else if let Some(id) = id_seguidor {
            estacion_addr.do_send(SeguidorCaidoMsg(id));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::Estacion;
    use actix::Actor;
    use std::collections::{HashMap, HashSet, VecDeque};
    use tp::coordenadas::Coordenadas;

    // Función constructora auxiliar (Helper) para inicializar una Estacion controlada y limpia
    fn crear_estacion_dummy(id: usize) -> Estacion {
        Estacion {
            id,
            nombre: format!("Estacion Test {}", id),
            slots: vec![],
            coordenadas: Coordenadas::new(0, 0),
            conectado: true,
            tx_tcp: None,
            otras_estaciones: HashSet::new(),
            lider_actual: None,
            procesador_de_pagos: "127.0.0.1:8080".parse().unwrap(),
            estaciones_info: vec![],
            ring_eleccion: None,
            servidor_tcp_iniciado: false,
            seguidores_tx: HashMap::new(),
            alquileres_activos: HashMap::new(),
            pagos_pendientes: VecDeque::new(),
        }
    }

    #[actix::test]
    async fn test01_eleccion_lider_new_con_exito() {
        let estacion = crear_estacion_dummy(9901);
        let addr = estacion.start();

        let eleccion = EleccionLider::new(9901, addr, true).await;
        assert!(
            eleccion.is_ok(),
            "Debería poder crear el EleccionLider correctamente"
        );

        let eleccion = eleccion.unwrap();
        assert!(
            eleccion.conectado.load(Ordering::SeqCst),
            "Debería inicializar como conectado"
        );
    }

    #[actix::test]
    async fn test02_eleccion_lider_new_puerto_en_uso() {
        let estacion = crear_estacion_dummy(9902);
        let addr = estacion.start();

        let _eleccion1 = EleccionLider::new(9902, addr.clone(), true)
            .await
            .expect("Debería inicializar bien la primera vez");
        let eleccion2 = EleccionLider::new(9902, addr, true).await;

        assert!(
            eleccion2.is_err(),
            "Debería fallar al inicializar porque el puerto UDP ya está ocupado por eleccion1"
        );
    }

    #[actix::test]
    async fn test03_cambiar_estado_conectividad() {
        let estacion = crear_estacion_dummy(9903);
        let addr = estacion.start();
        let eleccion = EleccionLider::new(9903, addr, true).await.unwrap();

        assert!(eleccion.conectado.load(Ordering::SeqCst));
        eleccion.cambiar_estado_de_conectividad(false);
        assert!(
            !eleccion.conectado.load(Ordering::SeqCst),
            "El estado atómico debería haber cambiado a false"
        );
    }

    #[actix::test]
    async fn test04_eliminar_utlimo_lider_coordinado() {
        let estacion = crear_estacion_dummy(9904);
        let addr = estacion.start();
        let eleccion = EleccionLider::new(9904, addr, true).await.unwrap();

        // Fuerza un líder ficticio
        eleccion.ultimo_lider_coordinado.store(5, Ordering::SeqCst);
        assert_eq!(eleccion.ultimo_lider_coordinado.load(Ordering::SeqCst), 5);

        // Verifica el reseteo
        eleccion.eliminar_utlimo_lider_coordinado();
        assert_eq!(
            eleccion.ultimo_lider_coordinado.load(Ordering::SeqCst),
            0,
            "El último líder coordinado debió volver a 0"
        );
    }

    #[actix::test]
    async fn test05_disparar_eleccion_sincrono_sin_panic() {
        let estacion = crear_estacion_dummy(9905);
        let addr = estacion.start();
        // Inicializamos como "desconectado" para que el método iniciar() retorne prematuramente y no inunde el puerto de paquetes
        let eleccion = EleccionLider::new(9905, addr, false).await.unwrap();

        // Simplemente verificamos que se puede invocar asíncronamente sin generar un hilo huérfano ni pánico
        eleccion.disparar_eleccion_sincrono();
    }
}
