//! actor.rs
//!
//! Módulo que contiene la definición del actor `UsuarioApp`, que modela el funcionamiento de la aplicación del usuario.
//!

use crate::config::DIR_ESTADO_USUARIOS;
use actix::{Actor, Context};
use std::{
    collections::HashMap,
    fs::{File, create_dir_all},
    io::{BufReader, BufWriter, Read, Write},
    net::{SocketAddr, TcpStream, UdpSocket},
    vec,
};
use tp::{
    constantes::{ADDR_BASE, PUERTO_BASE_APP_USUARIO, PUERTO_BASE_ESTACION, TIMEOUT_UDP},
    coordenadas::Coordenadas,
    msjs_app_usuario_estacion::{
        Bicicleta, BicicletaDevueltaCorrectamente, EntregarBicicleta, EnviarLiderActual,
        HayPedidoEnProcesoEnEseSlot, NoSePudoDevolverBicicletaEnSlot, NoTengoBicicletaEnEseSlot,
        ObtenerLiderActual, PagoRechazado, SolicitarEstado,
    },
    msjs_app_usuario_estacion_lider::{
        EstacionInfo, EstacionesPedidas, VisualizarEstadoEstaciones,
    },
    objetos_bancarios::TarjetaDeCredito,
};

/// Representa la aplicación del usuario, que se comunica con las estaciones para alquilar
/// y devolver bicicletas, y con la estación líder para obtener información sobre el estado
/// de las estaciones.
///
/// # Atributos
/// - `id`: Identificador númerico único del usuario.
/// - `coordenadas`: Coordenadas actuales del usuario.
/// - `estacion_lider`: ID de la estación líder asignada al usuario, si es que tiene una asignada.
/// - `tarjeta_de_credito`: Tarjeta de crédito del usuario, utilizada para realizar los pagos de los alquileres.
/// - `estaciones`: estaciones existentes en el sistema, con su ID, nombre, coordenadas y dirección.
/// - `bicicletas_en_uso`: Bicicletas que el usuario tiene actualmente en uso, con su ID y estructura que almacena su información.
pub struct UsuarioApp {
    pub id: usize,
    pub coordenadas: Coordenadas,
    pub estacion_lider: Option<usize>,
    pub tarjeta_de_credito: TarjetaDeCredito,
    pub estaciones: HashMap<usize, (String, Coordenadas, SocketAddr)>,
    pub bicicletas_en_uso: HashMap<usize, Bicicleta>,
    pub conectado: bool,
}

impl Actor for UsuarioApp {
    type Context = Context<Self>;
}

impl UsuarioApp {
    /// Crea una nueva instancia de `UsuarioApp` con los datos proporcionados.
    pub fn new(
        id: usize,
        coordenadas: Coordenadas,
        estacion_lider: Option<usize>,
        tarjeta_de_credito: TarjetaDeCredito,
        estaciones: HashMap<usize, (String, Coordenadas, SocketAddr)>,
        bicicletas_en_uso: HashMap<usize, Bicicleta>,
    ) -> Self {
        UsuarioApp {
            id,
            coordenadas,
            estacion_lider,
            tarjeta_de_credito,
            estaciones,
            bicicletas_en_uso,
            conectado: true,
        }
    }

    /// Recibe un mensajes relacionados al alquiler o devolución de una bicicleta, y se encarga de
    /// procesar la lógica correspondiente, enviando los mensajes necesarios a las estaciones y
    /// procesando las respuestas recibidas.
    ///
    /// Si la estación con la que se quiere comunicar no existe se imprime mensaje de error y termina.
    ///
    /// Si no se puede establecer conexión con la estación, se imprime mensaje de error y termina.
    ///
    /// Si se recibe una respuesta no reconocida, se imprime un mensaje de error y termina.
    pub fn procesar_alquiler_de_bicicleta(
        &mut self,
        mensaje: Vec<u8>,
        id_estacion: &usize,
        id_bicicleta_a_devolver: Option<usize>,
    ) {
        let Some((_, _, estacion_addr)) = self.estaciones.get(id_estacion) else {
            println!("Error: Estación con ID {} no encontrada.", id_estacion);
            return;
        };

        let Ok(stream) = TcpStream::connect(estacion_addr) else {
            eprintln!(
                "Error: No se pudo conectar a la estación en {}",
                estacion_addr
            );
            return;
        };

        let mut writer = BufWriter::new(&stream);
        let mut reader = BufReader::new(&stream);

        if let Err(e) = self.enviar_mensaje(mensaje, &mut writer) {
            eprintln!("Error al enviar mensaje a la estación: {:?}", e);
            return;
        }

        match self.recibir_mensaje(&mut reader) {
            Ok(bytes_respuesta) => {
                if let Ok(msg) = EntregarBicicleta::from_bytes(&bytes_respuesta) {
                    self.procesar_entregar_bicicleta(msg);
                } else if let Ok(msg) = NoTengoBicicletaEnEseSlot::from_bytes(&bytes_respuesta) {
                    self.procesar_no_tengo_bicicleta_en_ese_slot(msg);
                } else if let Ok(msg) = BicicletaDevueltaCorrectamente::from_bytes(&bytes_respuesta)
                {
                    self.procesar_bicicleta_devuelta_correctamente(msg, id_bicicleta_a_devolver);
                } else if let Ok(msg) =
                    NoSePudoDevolverBicicletaEnSlot::from_bytes(&bytes_respuesta)
                {
                    self.procesar_no_se_pudo_devolver_bicicleta_en_slot(msg);
                } else if let Ok(msg) = HayPedidoEnProcesoEnEseSlot::from_bytes(&bytes_respuesta) {
                    self.procesar_hay_pedido_en_proceso_en_ese_slot(msg);
                } else if let Ok(msg) = PagoRechazado::from_bytes(&bytes_respuesta) {
                    self.procesar_pago_rechazado(msg);
                } else {
                    println!("Respuesta no reconocida: {:?}", bytes_respuesta);
                }
            }
            Err(e) => {
                eprintln!("Error al recibir respuesta de la estación: {:?}", e);
            }
        }
    }

    /// Procesa el mensaje 'EntregarBicicleta', agregando la bicicleta a las bicicletas en uso
    /// del usuario y guardando el estado actualizado. Imprime un mensaje de éxito con el ID de
    /// la bicicleta entregada.
    fn procesar_entregar_bicicleta(&mut self, msg: EntregarBicicleta) {
        println!(
            "¡Éxito! Bicicleta con ID {} retirada correctamente.",
            msg.bicicleta.id
        );

        self.bicicletas_en_uso
            .insert(msg.bicicleta.id, msg.bicicleta);
        self.guardar_estado();
    }

    /// Procesa el mensaje 'NoTengoBicicletaEnEseSlot', imprimiendo un mensaje de error indicando
    /// que no hay bicicleta en el slot indicado.
    fn procesar_no_tengo_bicicleta_en_ese_slot(&self, msg: NoTengoBicicletaEnEseSlot) {
        println!("Error: No hay bicicleta en el slot {}.", msg.numero_slot);
    }

    /// Procesa el mensaje 'BicicletaDevueltaCorrectamente', eliminando la bicicleta devuelta de las
    /// bicicletas en uso del usuario, guardando el estado actualizado e imprimiendo un mensaje de
    /// éxito con el ID de la bicicleta devuelta.
    fn procesar_bicicleta_devuelta_correctamente(
        &mut self,
        _msg: BicicletaDevueltaCorrectamente,
        id_bicicleta_a_devolver: Option<usize>,
    ) {
        if let Some(id_bici) = id_bicicleta_a_devolver {
            self.bicicletas_en_uso.remove(&id_bici);
            self.guardar_estado();

            println!(
                "¡Éxito! Bicicleta con ID {} devuelta correctamente.",
                id_bici
            );
        } else {
            println!("¡Éxito! Bicicleta devuelta correctamente.");
        }
    }

    /// Procesa el mensaje 'NoSePudoDevolverBicicletaEnSlot', imprimiendo un mensaje de error
    /// indicando
    fn procesar_no_se_pudo_devolver_bicicleta_en_slot(&self, msg: NoSePudoDevolverBicicletaEnSlot) {
        println!(
            "Error: No se pudo devolver la bicicleta en el slot {}.",
            msg.numero_slot
        );
    }

    /// Procesa el mensaje 'HayPedidoEnProcesoEnEseSlot', imprimiendo un mensaje de error indicando que
    /// la bicicleta en el slot indicado se encuentra en proceso de pre-autorización a otro usuario.
    fn procesar_hay_pedido_en_proceso_en_ese_slot(&self, msg: HayPedidoEnProcesoEnEseSlot) {
        println!(
            "Error: La bicicleta en el slot {} se encuentra en proceso de pre-autorización a otro usuario, no se pudo iniciar su alquiler.",
            msg.numero_slot
        );
    }

    /// Procesa el mensaje 'PagoRechazado', imprimiendo un mensaje de error indicando que el pago
    /// fue rechazado.
    fn procesar_pago_rechazado(&self, _msg: PagoRechazado) {
        println!("Error: Pago rechazado.");
    }

    /// Envía un mensaje a la estación a través de un socket TCP, escribiendo primero la longitud del
    /// mensaje en bytes y luego el mensaje en sí. Si ocurre un error al escribir en el socket, se
    /// devuelve el error.
    fn enviar_mensaje(
        &self,
        mensaje: Vec<u8>,
        writer: &mut BufWriter<&TcpStream>,
    ) -> Result<(), std::io::Error> {
        let len = mensaje.len() as u32;
        writer.write_all(&len.to_be_bytes())?;
        writer.write_all(&mensaje)?;
        writer.flush()?;
        Ok(())
    }

    /// Recibe un mensaje de la estación a través de un socket TCP, leyendo primero la longitud
    /// del mensaje en bytes y luego el mensaje en sí. Si ocurre un error al leer del socket, se
    /// devuelve el error.
    pub fn recibir_mensaje(
        &self,
        reader: &mut BufReader<&TcpStream>,
    ) -> Result<Vec<u8>, std::io::Error> {
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut respuesta = vec![0u8; len];
        reader.read_exact(&mut respuesta)?;
        Ok(respuesta)
    }

    /// Envía una solicitud de estado de estación a la estación correspondiente, utilizando
    /// un socket UDP. Si ocurre un error al enviar el mensaje, se devuelve el error.
    pub fn enviar_solicitud_de_estado_estacion(
        &self,
        socket: &UdpSocket,
        id_estacion: usize,
    ) -> Result<(), std::io::Error> {
        self.enviar_mensaje_udp(socket, SolicitarEstado {}.as_bytes(), id_estacion)
    }

    /// Recibe la respuesta de estado de estación enviada por la estación, utilizando un socket UDP.
    /// Si ocurre un error al recibir el mensaje, se devuelve el error.
    pub fn recibir_respuesta_estado_estacion(
        &self,
        socket: &UdpSocket,
    ) -> Result<Vec<u8>, std::io::Error> {
        self.recibir_mensaje_udp(socket)
    }

    /// Ordena las estaciones por distancia desde las coordenadas actuales del usuario,
    /// devolviendo un vector con la información de cada estación (ID, nombre, coordenadas y
    /// distancia).
    pub fn ordenar_estaciones_por_distancia(&mut self) -> Vec<(usize, String, Coordenadas, f64)> {
        let mut estaciones: Vec<(usize, String, Coordenadas, f64)> = self
            .estaciones
            .iter()
            .map(|(id, (nombre, coordenadas, _))| {
                (
                    *id,
                    nombre.clone(),
                    *coordenadas,
                    self.coordenadas.distancia(*coordenadas),
                )
            })
            .collect();

        estaciones.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));
        estaciones
    }

    /// Obtiene el líder actual de las estaciones enviando solicitudes a las estaciones
    /// hasta recibir una respuesta válida con el líder asignado. Si se recibe una respuesta válida,
    /// se asigna el líder al usuario y se devuelve `true`. Si no se recibe ninguna respuesta válida
    /// de ninguna estación, se devuelve `false`.
    pub fn obtener_lider_actual(&mut self) -> bool {
        let addr = match format!("{}:{}", ADDR_BASE, PUERTO_BASE_APP_USUARIO + self.id as u16)
            .parse::<SocketAddr>()
        {
            Ok(addr) => addr,
            Err(e) => {
                eprintln!("Error al parsear la dirección del usuario: {:?}", e);
                return false;
            }
        };

        let socket: UdpSocket = match UdpSocket::bind(addr) {
            Ok(socket) => socket,
            Err(e) => {
                eprintln!("Error al crear el socket UDP: {:?}", e);
                return false;
            }
        };

        for estacion in self.ordenar_estaciones_por_distancia() {
            match self.enviar_solicitud_de_lider(&socket, estacion.0) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Error al enviar solicitud de líder: {:?}", e);
                    continue;
                }
            }
            match self.recibir_respuesta_con_lider(&socket) {
                Ok(lider) => {
                    println!("Lider actual: {}", lider.0);
                    self.estacion_lider = Some(lider.0);
                    return true;
                }
                Err(_) => {
                    continue;
                }
            }
        }
        false
    }

    /// Envía una solicitud a la estación para obtener el líder actual, utilizando un socket UDP.
    ///
    /// Si ocurre un error al enviar el mensaje, se devuelve el error.
    fn enviar_solicitud_de_lider(
        &mut self,
        socket: &UdpSocket,
        id_estacion: usize,
    ) -> Result<(), std::io::Error> {
        self.enviar_mensaje_udp(socket, ObtenerLiderActual.as_bytes(), id_estacion)
    }

    /// Recibe la respuesta con el líder actual de la estación, utilizando un socket UDP.
    ///
    /// Si ocurre un error al recibir el mensaje, se devuelve el error.
    fn recibir_respuesta_con_lider(
        &mut self,
        socket: &UdpSocket,
    ) -> Result<EnviarLiderActual, std::io::Error> {
        let bytes = self.recibir_mensaje_udp(socket)?;

        match EnviarLiderActual::from_bytes(&bytes) {
            Ok(msg) => Ok(msg),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Error al parsear EnviarLiderActual: {:?}", e),
            )),
        }
    }

    /// Solicita el estado de las estaciones especificadas al líder actual.
    ///
    /// Si no hay una estación líder asignada, se devuelve un error indicandolo.  
    pub fn solicitar_estado_estaciones(
        &mut self,
        ids_estaciones: &[usize],
    ) -> Result<Vec<(String, Coordenadas, EstacionInfo)>, std::io::Error> {
        let addr = match format!("{}:{}", ADDR_BASE, PUERTO_BASE_APP_USUARIO + self.id as u16)
            .parse::<SocketAddr>()
        {
            Ok(addr) => addr,
            Err(e) => {
                eprintln!("Error al parsear la dirección del usuario: {:?}", e);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Error al parsear la dirección del usuario",
                ));
            }
        };

        let socket: UdpSocket = UdpSocket::bind(addr)?;

        let lider_id = self.estacion_lider.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No hay una estación líder asignada",
            )
        })?;

        self.enviar_mensaje_udp(
            &socket,
            VisualizarEstadoEstaciones {
                estaciones: ids_estaciones.to_owned(),
            }
            .as_bytes(),
            lider_id,
        )?;

        let recibido = self.recibir_mensaje_udp(&socket)?;
        let respuesta = EstacionesPedidas::from_bytes(&recibido).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Error al parsear EstacionesPedidas",
            )
        })?;
        Ok(respuesta
            .estaciones
            .into_iter()
            .map(|estacion| {
                (
                    self.estaciones[&estacion.id].0.clone(),
                    self.estaciones[&estacion.id].1,
                    estacion,
                )
            })
            .collect())
    }

    /// Envía un mensaje a la estación a través de un socket UDP, escribiendo el mensaje en bytes.
    ///
    /// Si ocurre un error al escribir en el socket, se devuelve el error.
    fn enviar_mensaje_udp(
        &self,
        socket: &UdpSocket,
        mensaje: Vec<u8>,
        id_estacion: usize,
    ) -> Result<(), std::io::Error> {
        let addr = match format!(
            "{}:{}",
            ADDR_BASE,
            PUERTO_BASE_ESTACION + id_estacion as u16
        )
        .parse::<SocketAddr>()
        {
            Ok(addr) => addr,
            Err(e) => {
                eprintln!("Error al parsear la dirección de la estación: {:?}", e);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Error al parsear la dirección de la estación",
                ));
            }
        };
        socket.send_to(&mensaje, addr)?;
        Ok(())
    }

    /// Recibe un mensaje de la estación a través de un socket UDP.
    ///
    /// Si ocurre un error al leer del socket, se devuelve el error.
    fn recibir_mensaje_udp(&self, socket: &UdpSocket) -> Result<Vec<u8>, std::io::Error> {
        let mut buf = [0; 1024];
        socket.set_read_timeout(Some(TIMEOUT_UDP))?;
        let (leido, _) = socket.recv_from(&mut buf)?;
        Ok(buf[..leido].to_vec())
    }

    /// Guarda el estado actual del usuario en un archivo, incluyendo sus coordenadas, tarjeta de
    /// crédito y bicicletas en uso.
    ///
    /// Si ocurre un error al crear la carpeta de estados se imprime mensaje de error crítico y termina.
    ///
    /// Si ocurre un error al crear el archivo de estado se imprime mensaje de error y termina.
    ///
    /// Si ocurre un error al escribir en el archivo de estado se imprime mensaje de error y termina.
    ///
    /// # Formato:
    ///
    /// - Primera línea: coordenadas del usuario (longitud,latitud).
    /// - Segunda línea: bytes de la tarjeta de crédito del usuario, separados por comas.
    /// - Líneas siguientes: bytes de cada bicicleta en uso por el usuario, separados por
    ///   comas.
    ///
    /// # Ejemplo:
    /// ```state
    /// 0,0
    /// 54,50,52,53,54,54,49,53,51,56,48,56,48,51,50,54,3,44,48,51,47,50,52
    /// 0,0,0,0,0,0,0,201,1,0,0,0,0,106,46,208,168,0,0,0,0,0,0,0,1
    /// 0,0,0,0,0,0,0,202,1,0,0,0,0,106,46,208,172,0,0,0,0,0,0,0,1
    /// 0,0,0,0,0,0,0,102,1,0,0,0,0,106,46,208,167,0,0,0,0,0,0,0,1
    /// 0,0,0,0,0,0,0,103,1,0,0,0,0,106,46,208,204,0,0,0,0,0,0,0,1
    /// ```
    pub fn guardar_estado(&self) {
        if let Err(e) = create_dir_all(DIR_ESTADO_USUARIOS) {
            eprintln!(
                "[Estación {}] Error crítico al crear la carpeta de estados: {}",
                self.id, e
            );
            return;
        }

        let ruta_archivo = format!("{}/estado_usuario_{}.state", DIR_ESTADO_USUARIOS, self.id);

        if let Ok(mut archivo) = File::create(&ruta_archivo) {
            let coordenadas = format!(
                "{},{}",
                self.coordenadas.longitud(),
                self.coordenadas.latitud(),
            );
            if let Err(e) = writeln!(archivo, "{}", coordenadas) {
                eprintln!(
                    "[Usuario {}] Error al escribir en el archivo de estado: {}",
                    self.id, e
                );
            }

            let tarjeta_bytes = self
                .tarjeta_de_credito
                .as_bytes()
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<String>>()
                .join(",");
            if let Err(e) = writeln!(archivo, "{}", tarjeta_bytes) {
                eprintln!(
                    "[Usuario {}] Error al escribir en el archivo de estado: {}",
                    self.id, e
                );
            }

            for bicicleta in self.bicicletas_en_uso.values() {
                let bicicleta_bytes = bicicleta
                    .as_bytes()
                    .iter()
                    .map(|b| b.to_string())
                    .collect::<Vec<String>>()
                    .join(",");
                if let Err(e) = writeln!(archivo, "{}", bicicleta_bytes) {
                    eprintln!(
                        "[Usuario {}] Error al escribir en el archivo de estado: {}",
                        self.id, e
                    );
                }
            }
        } else {
            eprintln!("[Usuario {}] Error al crear el archivo de estado", self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tp::msjs_app_usuario_estacion::EstadoBicicleta;

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

    #[test]
    fn tests01_procesar_entregar_bicicleta() {
        let mut usuario_app = crear_usuario_app_dummy();
        let bicicleta = Bicicleta::new(123, EstadoBicicleta::Disponible);
        let msg = EntregarBicicleta {
            bicicleta: bicicleta.clone(),
        };
        usuario_app.procesar_entregar_bicicleta(msg);
        assert!(usuario_app.bicicletas_en_uso.contains_key(&bicicleta.id));
    }

    #[test]
    fn tests02_procesar_no_tengo_bicicleta_en_ese_slot() {
        let usuario_app = crear_usuario_app_dummy();
        let msg = NoTengoBicicletaEnEseSlot { numero_slot: 5 };
        usuario_app.procesar_no_tengo_bicicleta_en_ese_slot(msg);
        assert!(usuario_app.bicicletas_en_uso.is_empty());
    }

    #[test]
    fn tests03_procesar_bicicleta_devuelta_correctamente() {
        let mut usuario_app = crear_usuario_app_dummy();
        let bicicleta = Bicicleta::new(123, EstadoBicicleta::Disponible);
        usuario_app
            .bicicletas_en_uso
            .insert(bicicleta.id, bicicleta.clone());
        let msg = BicicletaDevueltaCorrectamente {};
        usuario_app.procesar_bicicleta_devuelta_correctamente(msg, Some(bicicleta.id));
        assert!(!usuario_app.bicicletas_en_uso.contains_key(&bicicleta.id));
    }

    #[test]
    fn tests04_procesar_no_se_pudo_devolver_bicicleta_en_slot() {
        let mut usuario_app = crear_usuario_app_dummy();
        let bicicleta = Bicicleta::new(123, EstadoBicicleta::Disponible);
        usuario_app
            .bicicletas_en_uso
            .insert(bicicleta.id, bicicleta.clone());
        let msg = NoSePudoDevolverBicicletaEnSlot { numero_slot: 5 };
        usuario_app.procesar_no_se_pudo_devolver_bicicleta_en_slot(msg);
        assert!(usuario_app.bicicletas_en_uso.contains_key(&bicicleta.id));
    }

    #[test]
    fn tests05_procesar_hay_pedido_en_proceso_en_ese_slot() {
        let usuario_app = crear_usuario_app_dummy();
        let msg = HayPedidoEnProcesoEnEseSlot { numero_slot: 5 };
        usuario_app.procesar_hay_pedido_en_proceso_en_ese_slot(msg);
        assert!(usuario_app.bicicletas_en_uso.is_empty());
    }

    #[test]
    fn tests06_ordenar_estaciones_por_distancia() {
        let mut usuario_app = crear_usuario_app_dummy();
        let estaciones_ordenadas = usuario_app.ordenar_estaciones_por_distancia();
        assert_eq!(estaciones_ordenadas[0].0, 1);
        assert_eq!(estaciones_ordenadas[1].0, 2);
        assert_eq!(estaciones_ordenadas[2].0, 3);
    }
}
