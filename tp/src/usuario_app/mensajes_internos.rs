use super::actor::UsuarioApp;
use actix::{Context, Handler, Message};
use std::net::{SocketAddr, UdpSocket};
use tp::{
    constantes::{ADDR_BASE, PUERTO_BASE_APP_USUARIO},
    coordenadas::Coordenadas,
    msjs_app_usuario_estacion::Bicicleta,
    msjs_app_usuario_estacion::{DevolverBicicleta, PedirBicicleta},
    msjs_app_usuario_estacion_lider::EstacionInfo,
    objetos_bancarios::TarjetaDeCredito,
};

/// Wrapper para poder tratar el mensaje de visualización de estaciones como un mensaje de Actix.
pub struct VerEstacionesExistentes;

impl Message for VerEstacionesExistentes {
    type Result = Vec<(usize, String, Coordenadas, f64)>;
}

impl Handler<VerEstacionesExistentes> for UsuarioApp {
    type Result = Vec<(usize, String, Coordenadas, f64)>;

    /// Retorna una lista con la información de todas las estaciones existentes, ordenadas por distancia a las coordenadas actuales del usuario.
    ///
    /// # Retornos
    /// - `Vec<(usize, String, Coordenadas, f64)>`: Lista de tuplas con el ID, nombre, coordenadas y distancia a cada estación, ordenada por distancia.
    fn handle(&mut self, _msg: VerEstacionesExistentes, _ctx: &mut Context<Self>) -> Self::Result {
        self.ordenar_estaciones_por_distancia()
    }
}

/// Wrapper para poder tratar el mensaje de listado de bicicletas en uso como un mensaje de Actix.
pub struct ListarBicicletasEnUso;

impl Message for ListarBicicletasEnUso {
    type Result = Vec<Bicicleta>;
}

impl Handler<ListarBicicletasEnUso> for UsuarioApp {
    type Result = Vec<Bicicleta>;

    /// Devuelve los IDs de las bicicletas que el usuario tiene en uso actualmente.
    ///
    /// # Retornos
    /// - `Vec<Bicicleta>`: Lista de bicicletas que el usuario tiene en uso actualmente.
    fn handle(&mut self, _msg: ListarBicicletasEnUso, _ctx: &mut Context<Self>) -> Self::Result {
        self.bicicletas_en_uso.values().cloned().collect()
    }
}

/// Wrapper para poder tratar el mensaje de solicitud de estado de estaciones como un mensaje de Actix.
///
/// # Atributos:
/// - `ids_estaciones`: Lista de IDs de las estaciones para las cuales se solicita el estado.
pub struct SolicitarEstadoEstaciones {
    pub ids_estaciones: Vec<usize>,
}

impl Message for SolicitarEstadoEstaciones {
    type Result = Option<Vec<(String, Coordenadas, EstacionInfo)>>;
}

impl Handler<SolicitarEstadoEstaciones> for UsuarioApp {
    type Result = Option<Vec<(String, Coordenadas, EstacionInfo)>>;

    /// Solicita el estado de las estaciones especificadas, enviando el mensaje correspondiente a cada estación.
    ///
    /// Si el usuario no tiene conexión, se muestra un mensaje de error indicandolo y se devuelve `None`.
    ///
    /// Si no se pudo determinar el líder actual del sistema, se muestra un mensaje de error indicandolo y se devuelve `None`.
    ///
    /// Si ocurre un error al solicitar el estado de las estaciones, se intenta obtener nuevamente el líder actual y repetir la solicitud.
    ///
    /// # Retornos
    /// - `Option<Vec<(String, Coordenadas, EstacionInfo)>>`: Lista de tuplas con el nombre, coordenadas e información de cada estación solicitada
    /// - None: si sucede algún error de los mencionados.
    fn handle(&mut self, msg: SolicitarEstadoEstaciones, _ctx: &mut Context<Self>) -> Self::Result {
        if !self.conectado {
            println!("Se debe poseer conexión para realizar esta acción.");
            return None;
        }

        if !self.obtener_lider_actual() {
            println!("No se pudo determinar el líder actual del sistema.");
            return None;
        }

        loop {
            match self.solicitar_estado_estaciones(&msg.ids_estaciones) {
                Ok(respuesta) => {
                    return Some(respuesta);
                }
                Err(_) => {
                    if !self.obtener_lider_actual() {
                        return None;
                    }
                }
            }
        }
    }
}

/// Wrapper para poder tratar el mensaje de solicitud de estado de una estación específica como un mensaje de Actix.
///
/// # Atributos:
/// - `id_estacion`: ID de la estación para la cual se solicita el estado.
pub struct SolicitarEstadoEstacion {
    pub id_estacion: usize,
}

impl Message for SolicitarEstadoEstacion {
    type Result = Result<Vec<u8>, std::io::Error>;
}

impl Handler<SolicitarEstadoEstacion> for UsuarioApp {
    type Result = Result<Vec<u8>, std::io::Error>;

    /// Solicita el estado de una estación específica, enviando el mensaje correspondiente a la estación.
    ///
    /// # Retornos
    /// - `Ok(Vec<u8>)`: Respuesta recibida de la estación con su estado, en bytes.
    /// - `Err(std::io::Error)`: Si ocurre un error al enviar la solicitud o al recibir la respuesta de la estación.
    fn handle(&mut self, msg: SolicitarEstadoEstacion, _ctx: &mut Context<Self>) -> Self::Result {
        let addr = match format!("{}:{}", ADDR_BASE, PUERTO_BASE_APP_USUARIO + self.id as u16)
            .parse::<SocketAddr>()
        {
            Ok(addr) => addr,
            Err(e) => {
                eprintln!(
                    "\x1b[31mError al parsear la dirección para solicitar el estado de la estación: {}\x1b[0m",
                    e
                );
                return Err(std::io::Error::other(
                    "Error al parsear la dirección para solicitar el estado de la estación.",
                ));
            }
        };

        let socket = match UdpSocket::bind(addr) {
            Ok(socket) => socket,
            Err(e) => {
                eprintln!(
                    "\x1b[31mError al crear el socket para solicitar el estado de la estación: {}\x1b[0m",
                    e
                );
                return Err(e);
            }
        };
        self.enviar_solicitud_de_estado_estacion(&socket, msg.id_estacion)?;
        self.recibir_respuesta_estado_estacion(&socket)
    }
}

/// Wrapper para poder tratar el mensaje de inicio de alquiler de bicicleta como un mensaje de Actix.
///
/// # Atributos:
/// - `id_estacion`: ID de la estación donde se desea iniciar el alquiler.
/// - `num_slot`: Número del slot que contiene la bicicleta que se desea alquilar.
pub struct IniciarAlquilerBicicleta {
    pub id_estacion: usize,
    pub num_slot: u8,
}

impl Message for IniciarAlquilerBicicleta {
    type Result = ();
}

impl Handler<IniciarAlquilerBicicleta> for UsuarioApp {
    type Result = ();

    /// Inicia el alquiler de una bicicleta, enviando el mensaje correspondiente a la estación.
    fn handle(&mut self, msg: IniciarAlquilerBicicleta, _ctx: &mut Context<Self>) -> Self::Result {
        let mensaje = PedirBicicleta {
            id: self.id,
            numero_slot: msg.num_slot,
            tarjeta_de_credito: self.tarjeta_de_credito.clone(),
        }
        .as_bytes();

        println!(
            "Iniciando alquiler de bicicleta en estación {} slot {}...",
            msg.id_estacion, msg.num_slot
        );
        self.procesar_alquiler_de_bicicleta(mensaje, &msg.id_estacion, None);
    }
}

/// Wrapper para poder tratar el mensaje de finalización de alquiler de bicicleta como un mensaje de Actix.
///
/// # Atributos:
/// - `id_estacion`: ID de la estación donde se desea devolver la bicicleta.
/// - `num_slot`: Número del slot de la estación donde se desea devolver la bicicleta.
/// - `id_bicicleta`: ID de la bicicleta que se desea devolver.
pub struct FinalizarAlquilerBicicleta {
    pub id_estacion: usize,
    pub num_slot: u8,
    pub id_bicicleta: usize,
}

impl Message for FinalizarAlquilerBicicleta {
    type Result = ();
}

impl Handler<FinalizarAlquilerBicicleta> for UsuarioApp {
    type Result = ();

    /// Finaliza el alquiler de una bicicleta, enviando el mensaje correspondiente a la estación.
    ///
    /// Si el usuario no tiene en uso la bicicleta que se desea devolver, se muestra un mensaje de error indicandolo.
    fn handle(
        &mut self,
        msg: FinalizarAlquilerBicicleta,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        let bicicleta = match self.bicicletas_en_uso.get(&msg.id_bicicleta) {
            Some(bici) => bici.clone(),
            None => {
                println!(
                    "Error: No tenés en uso la bicicleta con ID {}.",
                    msg.id_bicicleta
                );
                return;
            }
        };

        let mensaje = DevolverBicicleta {
            id: self.id,
            numero_slot: msg.num_slot,
            tarjeta_de_credito: self.tarjeta_de_credito.clone(),
            bicicleta,
        }
        .as_bytes();

        println!(
            "Finalizando alquiler de bicicleta en estación {} slot {}...",
            msg.id_estacion, msg.num_slot
        );
        self.procesar_alquiler_de_bicicleta(mensaje, &msg.id_estacion, Some(msg.id_bicicleta));
    }
}

/// Wrapper para poder tratar el mensaje de actualización de coordenadas como un mensaje de Actix.
///
/// # Atributos:
/// - `coordenadas`: Nuevas coordenadas del usuario a actualizar.
pub struct ActualizarCoordenadas {
    pub coordenadas: Coordenadas,
}

impl Message for ActualizarCoordenadas {
    type Result = ();
}

impl Handler<ActualizarCoordenadas> for UsuarioApp {
    type Result = ();

    /// Actualiza las coordenadas actuales del usuario.
    fn handle(&mut self, msg: ActualizarCoordenadas, _ctx: &mut Context<Self>) -> Self::Result {
        self.coordenadas = msg.coordenadas;
    }
}

/// Wrapper para poder tratar el mensaje de visualización de información del usuario como un mensaje de Actix.
pub struct VisualizarInfoUsuario;

impl Message for VisualizarInfoUsuario {
    type Result = Option<(usize, Coordenadas, TarjetaDeCredito, bool, usize)>;
}

impl Handler<VisualizarInfoUsuario> for UsuarioApp {
    type Result = Option<(usize, Coordenadas, TarjetaDeCredito, bool, usize)>;

    /// Retorna la información del usuario, incluyendo su ID, coordenadas actuales y tarjeta de crédito.
    ///
    /// # Retornos:
    /// - `Some((usize, Coordenadas, TarjetaDeCredito, bool, usize))`: Tupla con el ID del usuario, sus coordenadas actuales, su tarjeta de crédito, su estado de conectividad y la cantidad de bicicletas que tiene en uso actualmente.
    fn handle(&mut self, _msg: VisualizarInfoUsuario, _ctx: &mut Context<Self>) -> Self::Result {
        Some((
            self.id,
            self.coordenadas,
            self.tarjeta_de_credito.clone(),
            self.conectado,
            self.bicicletas_en_uso.len(),
        ))
    }
}

/// Wrapper para poder tratar el mensaje de cambio de conectividad como un mensaje de Actix.
pub struct CambiarConectividad;

impl Message for CambiarConectividad {
    type Result = ();
}

impl Handler<CambiarConectividad> for UsuarioApp {
    type Result = ();

    /// Cambia el estado de conectividad del usuario.
    fn handle(&mut self, _msg: CambiarConectividad, _ctx: &mut Context<Self>) -> Self::Result {
        self.conectado = !self.conectado;

        if self.conectado {
            println!("Conexión activada correctamente.");
        } else {
            self.estacion_lider = None;
            println!("Conexión desactivada correctamente.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix::Actor;
    use std::{
        collections::HashMap,
        io::{Read, Write},
        net::{TcpListener, TcpStream, UdpSocket},
        thread,
    };
    use tp::{
        constantes::PUERTO_BASE_ESTACION, msjs_app_usuario_estacion::*,
        msjs_app_usuario_estacion_lider::*,
    };

    fn crear_usuario_app_dummy() -> UsuarioApp {
        let mut estaciones = HashMap::new();
        estaciones.insert(
            1,
            (
                "Estación 1".to_string(),
                Coordenadas::new(0, 0),
                format!("{}:{}", ADDR_BASE, PUERTO_BASE_ESTACION + 1)
                    .parse()
                    .unwrap(),
            ),
        );
        estaciones.insert(
            2,
            (
                "Estación 2".to_string(),
                Coordenadas::new(1, 1),
                format!("{}:{}", ADDR_BASE, PUERTO_BASE_ESTACION + 2)
                    .parse()
                    .unwrap(),
            ),
        );
        estaciones.insert(
            3,
            (
                "Estación 3".to_string(),
                Coordenadas::new(2, 2),
                format!("{}:{}", ADDR_BASE, PUERTO_BASE_ESTACION + 3)
                    .parse()
                    .unwrap(),
            ),
        );
        UsuarioApp::new(
            1,
            Coordenadas::new(0, 0),
            Some(1),
            TarjetaDeCredito::new("1234567890123456", 123, "12/25"),
            estaciones,
            HashMap::new(),
        )
    }

    fn crear_estacion_dummy(
        id_estacion: usize,
        usuario_app: &mut UsuarioApp,
    ) -> (String, Coordenadas, String) {
        let nombre = format!("Estación {}", id_estacion);
        let coordenadas = Coordenadas::new(id_estacion as isize, id_estacion as isize);
        let addr = format!("127.0.0.1:{}", PUERTO_BASE_ESTACION + id_estacion as u16);

        usuario_app.estaciones.insert(
            id_estacion,
            (nombre.clone(), coordenadas, addr.parse().unwrap()),
        );

        (nombre, coordenadas, addr)
    }

    fn enviar_mensaje_tcp(stream: &mut TcpStream, bytes: &[u8]) -> std::io::Result<()> {
        let len = bytes.len() as u32;
        stream.write_all(&len.to_be_bytes())?;
        stream.write_all(bytes)?;
        stream.flush()?;
        Ok(())
    }

    #[actix::test]
    async fn test01_ver_estaciones_existentes() {
        let usuario_app = crear_usuario_app_dummy();
        let addr = usuario_app.start();
        let resultado = addr.send(VerEstacionesExistentes).await.unwrap();
        assert_eq!(resultado.len(), 3);
        assert_eq!(resultado[0].0, 1);
        assert_eq!(resultado[1].0, 2);
        assert_eq!(resultado[2].0, 3);
    }

    #[actix::test]
    async fn test02_listar_bicicletas_en_uso() {
        let mut usuario_app = crear_usuario_app_dummy();
        for i in 0..3 {
            usuario_app
                .bicicletas_en_uso
                .insert(i, Bicicleta::new(i, EstadoBicicleta::Disponible));
        }
        let addr = usuario_app.start();
        let resultado = addr.send(ListarBicicletasEnUso).await.unwrap();
        let ids_bicicletas: Vec<usize> = resultado.iter().map(|bici| bici.id).collect();
        assert_eq!(resultado.len(), 3);
        assert!(ids_bicicletas.contains(&0));
        assert!(ids_bicicletas.contains(&1));
        assert!(ids_bicicletas.contains(&2));
    }

    #[actix::test]
    async fn test04_solicitar_estado_estaciones_sin_conexion() {
        let mut usuario_app = crear_usuario_app_dummy();
        usuario_app.conectado = false;
        let addr = usuario_app.start();
        let resultado = addr
            .send(SolicitarEstadoEstaciones {
                ids_estaciones: vec![1, 2],
            })
            .await
            .unwrap();
        assert!(resultado.is_none());
    }

    #[actix::test]
    async fn test05_solicitar_estado_estaciones_inexistente() {
        let usuario_app = crear_usuario_app_dummy();
        let addr = usuario_app.start();
        let resultado = addr
            .send(SolicitarEstadoEstaciones {
                ids_estaciones: vec![999],
            })
            .await
            .unwrap();
        assert!(resultado.is_none());
    }

    #[actix::test]
    async fn test06_solicitar_estado_estaciones() {
        let mut usuario_app = crear_usuario_app_dummy();
        usuario_app.id = 6;

        let id_estacion = 6;
        let slots_libres = 5;
        let slots_ocupados = 10;
        let (nombre_estacion, coordenadas_estacion, addr) =
            crear_estacion_dummy(id_estacion, &mut usuario_app);
        let socket = UdpSocket::bind(addr).unwrap();

        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            let mut intento = 0;
            let max_intentos = 10;

            loop {
                if let Ok((len, addr)) = socket.recv_from(&mut buf) {
                    let recibido = &buf[..len];

                    if ObtenerLiderActual::from_bytes(recibido).is_ok() {
                        let respuesta = EnviarLiderActual(id_estacion).as_bytes();
                        let _ = socket.send_to(&respuesta, addr).unwrap();
                    } else if VisualizarEstadoEstaciones::from_bytes(recibido).is_ok() {
                        let info = EstacionInfo {
                            id: id_estacion,
                            slots_libres,
                            slots_ocupados,
                            estado: EstacionEstado::Conectada,
                        };

                        let respuesta = EstacionesPedidas {
                            estaciones: vec![info],
                        }
                        .as_bytes();
                        let _ = socket.send_to(&respuesta, addr).unwrap();
                    }
                }
                intento += 1;
                if intento >= max_intentos {
                    panic!(
                        "La estación de prueba no recibió las solicitudes esperadas después de {} intentos.",
                        max_intentos
                    );
                }
            }
        });

        let addr = usuario_app.start();

        let resultado = addr
            .send(SolicitarEstadoEstaciones {
                ids_estaciones: vec![id_estacion],
            })
            .await
            .unwrap();
        assert!(resultado.is_some());

        let estados = resultado.unwrap();
        assert_eq!(estados.len(), 1);
        assert_eq!(estados[0].0, nombre_estacion);
        assert_eq!(estados[0].1, coordenadas_estacion);
        assert_eq!(estados[0].2.id, id_estacion);
        assert_eq!(estados[0].2.slots_libres, slots_libres);
        assert_eq!(estados[0].2.slots_ocupados, slots_ocupados);
        assert_eq!(estados[0].2.estado, EstacionEstado::Conectada);
    }

    #[actix::test]
    async fn test07_solicitar_estado_estacion() {
        let mut usuario_app = crear_usuario_app_dummy();
        usuario_app.id = 7;

        let id_estacion = 7;
        let (_, _, addr) = crear_estacion_dummy(id_estacion, &mut usuario_app);
        let socket = UdpSocket::bind(addr).unwrap();

        let respuesta_dummy_a_enviar = vec![1, 2, 3, 4];
        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            if let Ok((len, addr)) = socket.recv_from(&mut buf) {
                let recibido = &buf[..len];
                if SolicitarEstado::from_bytes(recibido).is_ok() {
                    let _ = socket
                        .send_to(&respuesta_dummy_a_enviar.clone(), addr)
                        .unwrap();
                } else {
                    panic!("La estación de prueba no recibió la solicitud esperada.");
                }
            }
        });

        let respuesta_dummy_a_recibir = vec![1, 2, 3, 4];
        let addr = usuario_app.start();
        let resultado = addr
            .send(SolicitarEstadoEstacion { id_estacion })
            .await
            .unwrap();
        assert!(resultado.is_ok());

        let estado = resultado.unwrap();
        assert_eq!(estado, respuesta_dummy_a_recibir);
    }

    #[actix::test]
    async fn test08_iniciar_alquiler_bicicleta_en_slot_vacio() {
        let mut usuario_app = crear_usuario_app_dummy();
        usuario_app.id = 8;

        let id_estacion = 8;
        let (_, _, addr) = crear_estacion_dummy(id_estacion, &mut usuario_app);
        let listener = TcpListener::bind(addr).unwrap();

        let numero_slot = 1;

        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            if let Ok((mut stream, _)) = listener.accept() {
                if let Ok(bytes_leidos) = stream.read(&mut buf) {
                    let recibido = &buf[4..bytes_leidos];
                    if PedirBicicleta::from_bytes(recibido).is_ok() {
                        let respuesta = NoTengoBicicletaEnEseSlot { numero_slot }.as_bytes();
                        enviar_mensaje_tcp(&mut stream, &respuesta).unwrap();
                    } else {
                        panic!("La estación de prueba no recibió la solicitud esperada.");
                    }
                } else {
                    panic!("La estación de prueba no pudo leer la solicitud recibida.");
                }
            }
        });

        let addr = usuario_app.start();
        addr.send(IniciarAlquilerBicicleta {
            id_estacion,
            num_slot: numero_slot,
        })
        .await
        .unwrap();
        let (_, _, _, _, cant_bicicletas) =
            addr.send(VisualizarInfoUsuario).await.unwrap().unwrap();
        assert_eq!(cant_bicicletas, 0);
    }

    #[actix::test]
    async fn test09_iniciar_alquiler_bicicleta_en_slot_ocupado() {
        let mut usuario_app = crear_usuario_app_dummy();
        usuario_app.id = 9;

        let id_estacion = 9;
        let (_, _, addr) = crear_estacion_dummy(id_estacion, &mut usuario_app);
        let listener = TcpListener::bind(addr).unwrap();

        let numero_slot = 1;
        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            if let Ok((mut stream, _)) = listener.accept() {
                if let Ok(bytes_leidos) = stream.read(&mut buf) {
                    let recibido = &buf[4..bytes_leidos];
                    if PedirBicicleta::from_bytes(recibido).is_ok() {
                        let respuesta = EntregarBicicleta {
                            bicicleta: Bicicleta::new(1, EstadoBicicleta::Disponible),
                        }
                        .as_bytes();
                        enviar_mensaje_tcp(&mut stream, &respuesta).unwrap();
                    } else {
                        panic!("La estación de prueba no recibió la solicitud esperada.");
                    }
                } else {
                    panic!("La estación de prueba no pudo leer la solicitud recibida.");
                }
            }
        });

        let addr = usuario_app.start();
        addr.send(IniciarAlquilerBicicleta {
            id_estacion,
            num_slot: numero_slot,
        })
        .await
        .unwrap();
        let bicicletas_en_uso = addr.send(ListarBicicletasEnUso).await.unwrap();
        assert_eq!(bicicletas_en_uso.len(), 1);
        assert_eq!(bicicletas_en_uso[0].id, 1);
    }

    #[actix::test]
    async fn test10_finalizar_alquieler_bicicleta_en_slot_vacio() {
        let mut usuario_app = crear_usuario_app_dummy();
        usuario_app.id = 9;
        usuario_app
            .bicicletas_en_uso
            .insert(1, Bicicleta::new(1, EstadoBicicleta::Disponible));

        let id_estacion = 9;
        let (_, _, addr) = crear_estacion_dummy(id_estacion, &mut usuario_app);
        let listener = TcpListener::bind(addr).unwrap();

        let numero_slot = 1;
        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            if let Ok((mut stream, _)) = listener.accept() {
                if let Ok(bytes_leidos) = stream.read(&mut buf) {
                    let recibido = &buf[4..bytes_leidos];
                    if DevolverBicicleta::from_bytes(recibido).is_ok() {
                        let respuesta = BicicletaDevueltaCorrectamente.as_bytes();
                        enviar_mensaje_tcp(&mut stream, &respuesta).unwrap();
                    } else {
                        panic!("La estación de prueba no recibió la solicitud esperada.");
                    }
                } else {
                    panic!("La estación de prueba no pudo leer la solicitud recibida.");
                }
            }
        });

        let addr = usuario_app.start();
        addr.send(FinalizarAlquilerBicicleta {
            id_estacion,
            num_slot: numero_slot,
            id_bicicleta: 1,
        })
        .await
        .unwrap();
        let bicicletas_en_uso = addr.send(ListarBicicletasEnUso).await.unwrap();
        assert_eq!(bicicletas_en_uso.len(), 0);
    }

    #[actix::test]
    async fn test11_finalizar_alquieler_bicicleta_en_slot_ocupado() {
        let mut usuario_app = crear_usuario_app_dummy();
        usuario_app.id = 9;
        usuario_app
            .bicicletas_en_uso
            .insert(1, Bicicleta::new(1, EstadoBicicleta::Disponible));

        let id_estacion = 9;
        let (_, _, addr) = crear_estacion_dummy(id_estacion, &mut usuario_app);
        let listener = TcpListener::bind(addr).unwrap();

        let numero_slot = 1;
        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            if let Ok((mut stream, _)) = listener.accept() {
                if let Ok(bytes_leidos) = stream.read(&mut buf) {
                    let recibido = &buf[4..bytes_leidos];
                    if DevolverBicicleta::from_bytes(recibido).is_ok() {
                        let respuesta = NoSePudoDevolverBicicletaEnSlot { numero_slot }.as_bytes();
                        enviar_mensaje_tcp(&mut stream, &respuesta).unwrap();
                    } else {
                        panic!("La estación de prueba no recibió la solicitud esperada.");
                    }
                } else {
                    panic!("La estación de prueba no pudo leer la solicitud recibida.");
                }
            }
        });

        let addr = usuario_app.start();
        addr.send(FinalizarAlquilerBicicleta {
            id_estacion,
            num_slot: numero_slot,
            id_bicicleta: 1,
        })
        .await
        .unwrap();
        let bicicletas_en_uso = addr.send(ListarBicicletasEnUso).await.unwrap();
        assert_eq!(bicicletas_en_uso.len(), 1);
    }

    #[actix::test]
    async fn test12_actualizar_coordenadas() {
        let usuario_app = crear_usuario_app_dummy();
        let addr = usuario_app.start();
        let nuevas_coordenadas = Coordenadas::new(5, 5);
        addr.send(ActualizarCoordenadas {
            coordenadas: nuevas_coordenadas,
        })
        .await
        .unwrap();
        let resultado = addr.send(VisualizarInfoUsuario).await.unwrap();
        assert!(resultado.is_some());
        let (_, coordenadas_actualizadas, _, _, _) = resultado.unwrap();
        assert_eq!(coordenadas_actualizadas, nuevas_coordenadas);
    }

    #[actix::test]
    async fn test13_visualizar_info_usuario() {
        let usuario_app = crear_usuario_app_dummy();
        let addr = usuario_app.start();
        let resultado = addr.send(VisualizarInfoUsuario).await.unwrap();
        assert!(resultado.is_some());
        let (id, coordenadas, tarjeta, conectado, cant_bicicletas) = resultado.unwrap();
        assert_eq!(id, 1);
        assert_eq!(coordenadas, Coordenadas::new(0, 0));
        assert_eq!(tarjeta.numero, "1234567890123456");
        assert_eq!(tarjeta.cod_seguridad, 123);
        assert_eq!(tarjeta.vencimiento, "12/25");
        assert_eq!(cant_bicicletas, 0);
        assert!(conectado);
    }

    #[actix::test]
    async fn test14_cambiar_conectividad() {
        let usuario_app = crear_usuario_app_dummy();
        let addr = usuario_app.start();
        let resultado_ant = addr.send(VisualizarInfoUsuario).await.unwrap();
        assert!(resultado_ant.is_some());
        let (_, _, _, conectado_ant, _) = resultado_ant.unwrap();
        assert!(conectado_ant);
        addr.send(CambiarConectividad).await.unwrap();
        let resultado_desp = addr.send(VisualizarInfoUsuario).await.unwrap();
        assert!(resultado_desp.is_some());
        let (_, _, _, conectado_desp, _) = resultado_desp.unwrap();
        assert!(!conectado_desp);
    }
}
