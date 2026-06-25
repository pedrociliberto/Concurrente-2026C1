//! actor.rs
//!
//! Módulo que contiene la definición del actor `Estacion` y su lódica asociada.
//!

use crate::{
    MONTO_DE_SEGURIDAD,
    eleccion_de_lider::EleccionLider,
    mensajes_internos::{MensajeEntranteTcpMsg, RegistrarAlquilerActivoMsg},
};
use actix::{Actor, AsyncContext, Context};
use std::fs::File;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs::create_dir_all,
    io::Write,
    net::SocketAddr,
    time::Duration,
    time::Instant,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc,
};
use tp::{
    constantes::TIEMPO_MAX_PRE_ROBO,
    coordenadas::Coordenadas,
    msjs_app_usuario_estacion::{Bicicleta, EstadoBicicleta, PagoRechazado},
    msjs_app_usuario_estacion_lider::{EstacionEstado, EstacionInfo},
    objetos_bancarios::TarjetaDeCredito,
};

/// Representa los posibles estados en los que puede encontrarse un slot de estacionamiento.
#[derive(Clone, Debug)]
pub enum EstadoSlot {
    /// El slot se encuentra libre y listo para recibir una bicicleta.
    Vacio,
    /// El slot contiene una bicicleta que puede estar disponible o en uso (físicamente retenida).
    Ocupado(Bicicleta),
    /// Estado transitorio (Fase 1 de 2PC) que bloquea el slot mientras se autoriza el pago.
    /// Almacena la bicicleta, el ID del usuario solicitante y el canal para enviarle la respuesta asíncrona.
    PreparandoRetiro(Bicicleta, usize, mpsc::Sender<Vec<u8>>),
}

impl PartialEq for EstadoSlot {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (EstadoSlot::Vacio, EstadoSlot::Vacio) => true,
            (EstadoSlot::Ocupado(b1), EstadoSlot::Ocupado(b2)) => b1 == b2,
            (
                EstadoSlot::PreparandoRetiro(b1, id1, _),
                EstadoSlot::PreparandoRetiro(b2, id2, _),
            ) => b1 == b2 && id1 == id2,
            _ => false,
        }
    }
}

/// Representa una estación física la cual alberga los slots, que contienen las bicicletas que los usuarios pueden retirar o
/// entregar bicicletas ya previamente retiradas.
///
/// # Atributos
/// - `id`: Identificador numérico único de la estación.
/// - `nombre`: Nombre legible de la estación.
/// - `slots`: Colección de `EstadoSlot` que representa los lugares físicos.
/// - `coordenadas`: Ubicación geográfica (latitud, longitud).
/// - `conectado`: Indica si la estación tiene conexión de red activa (simulada).
/// - `tx_tcp`: Canal para enviar mensajes TCP salientes (hacia el líder) si la estación es seguidora.
/// - `otras_estaciones`: Conjunto con los IDs del resto de estaciones de la red.
/// - `lider_actual`: ID del líder coordinador del sistema. `None` si la red está en proceso de elección.
/// - `procesador_de_pagos`: Dirección socket donde atiende el servidor de pagos.
/// - `estaciones_info`: Caché del estado general de la red. Solo es mantenido actualizado si la estación actual es líder.
/// - `ring_eleccion`: Instancia del manejador del protocolo de elección por anillo UDP.
/// - `servidor_tcp_iniciado`: Indica si el proceso de escucha TCP para nodos seguidores está activo.
/// - `seguidores_tx`: Mapa de canales para enviar comandos a los nodos seguidores (si actúa como líder).
/// - `alquileres_activos`: Mapa de los alquileres actualmente en curso iniciados en esta estación.
/// - `pagos_pendientes`: Cola de comandos (textuales) de pago que esperan ser despachados al líder cuando se recupere la conexión.
pub struct Estacion {
    pub id: usize,
    pub nombre: String,
    pub slots: Vec<EstadoSlot>,
    pub coordenadas: Coordenadas,
    pub conectado: bool,
    pub tx_tcp: Option<mpsc::Sender<String>>,

    pub otras_estaciones: HashSet<usize>,
    pub lider_actual: Option<usize>,
    pub procesador_de_pagos: SocketAddr,
    pub estaciones_info: Vec<EstacionInfo>,
    pub ring_eleccion: Option<EleccionLider>,
    pub servidor_tcp_iniciado: bool,
    pub seguidores_tx: HashMap<usize, mpsc::Sender<String>>,
    pub alquileres_activos: HashMap<usize, Vec<(usize, Instant, TarjetaDeCredito)>>,
    pub pagos_pendientes: VecDeque<String>,
}

impl Estacion {
    /// Comprueba que el índice de slot proporcionado se encuentre dentro de los límites del vector de slots.
    ///
    /// # Parámetros
    /// - `numero_slot`: El índice en base 0 del slot a verificar.
    ///
    /// # Retornos
    /// - `bool`: `true` si es válido, `false` de lo contrario.
    pub fn assert_numero_slot_valido(&self, numero_slot: usize) -> bool {
        if numero_slot >= self.slots.len() {
            println!("Número de slot inválido: {}", numero_slot + 1);
            return false;
        }
        true
    }

    /// Contabiliza el estado de sus propios slots y notifica al líder sobre su disponibilidad.
    /// Si la estación misma es el líder, directamente actualiza su caché interno (`estaciones_info`).
    /// Si se encuentra desconectada, omite la acción.
    pub fn notificar_cambio_slots_al_lider(&mut self) {
        if !self.conectado {
            return;
        }

        let slots_libres = self
            .slots
            .iter()
            .filter(|s| matches!(s, EstadoSlot::Vacio))
            .count();
        let slots_ocupados = self.slots.len() - slots_libres;

        if self.lider_actual == Some(self.id) {
            if let Some(mi_info) = self.estaciones_info.iter_mut().find(|e| e.id == self.id) {
                mi_info.slots_libres = slots_libres;
                mi_info.slots_ocupados = slots_ocupados;
                mi_info.estado = tp::msjs_app_usuario_estacion_lider::EstacionEstado::Conectada;
            }
            return;
        }

        if let Some(ref tx) = self.tx_tcp {
            let mensaje_estado = format!(
                "INFO_ESTACION:{}:{}:{}",
                self.id, slots_libres, slots_ocupados
            );

            let tx_clone = tx.clone();
            let id_estacion = self.id;
            tokio::spawn(async move {
                if let Err(e) = tx_clone.send(mensaje_estado).await {
                    eprintln!(
                        "[Estación {}] Error al enviar actualización de slots al líder: {}",
                        id_estacion, e
                    );
                }
            });
        }
    }

    /// Serializa el estado de ocupación actual de los slots y lo persiste en un archivo `.state` en disco.
    /// Esto permite a la estación recuperar su infraestructura (y viajes temporalmente bloqueados) si el proceso se reinicia.
    pub fn guardar_estado_en_disco(&self) {
        let carpeta = if cfg!(test) {
            "src/test_estado_estaciones"
        } else {
            "src/estado_estaciones"
        };
        if let Err(e) = create_dir_all(carpeta) {
            eprintln!(
                "[Estación {}] Error crítico al crear la carpeta de estados: {}",
                self.id, e
            );
            return;
        }
        let nombre_archivo = format!("{}/estacion_{}.state", carpeta, self.id);
        if let Ok(mut archivo) = File::create(&nombre_archivo) {
            for slot in &self.slots {
                match slot {
                    EstadoSlot::Vacio => {
                        let _ = writeln!(archivo, "VACIO");
                    }
                    EstadoSlot::Ocupado(bici) => match &bici.estado {
                        EstadoBicicleta::Disponible => {
                            let _ = writeln!(archivo, "OCUPADO,{},Disponible", bici.id);
                        }
                        EstadoBicicleta::EnUso(inicio_uso, id_usuario) => {
                            let _ = writeln!(
                                archivo,
                                "OCUPADO,{},EnUso,{},{:?}",
                                bici.id, id_usuario, inicio_uso
                            );
                        }
                    },
                    EstadoSlot::PreparandoRetiro(bici, id_usuario, sender) => {
                        let _ =
                            writeln!(archivo, "PREPARE,{},{},{:?}", bici.id, id_usuario, sender);
                    }
                }
            }
        }
    }

    /// Persiste la tabla de alquileres activos en disco (archivo `alquileres.state`).
    /// Almacena el ID del usuario, la bicicleta arrendada, los segundos transcurridos y los datos de la tarjeta.
    pub fn guardar_alquileres_en_disco(&self) {
        let carpeta = if cfg!(test) {
            "src/test_estado_estaciones"
        } else {
            "src/estado_estaciones"
        };
        if let Err(e) = create_dir_all(carpeta) {
            eprintln!(
                "[Estación {}] Error crítico al crear la carpeta de estados: {}",
                self.id, e
            );
            return;
        }
        let nombre_archivo = format!("{}/alquileres.state", carpeta);
        if let Ok(mut archivo) = File::create(&nombre_archivo) {
            for (id_usuario, viajes) in &self.alquileres_activos {
                for (id_bici, inicio, tarjeta) in viajes {
                    let segundos = inicio.elapsed().as_secs();
                    let _ = writeln!(
                        archivo,
                        "{},{},{},{:?}",
                        id_usuario, id_bici, segundos, tarjeta
                    );
                }
            }
        }
    }

    /// Procesa el mensaje textual "INFO_ESTACION" proveniente de un nodo seguidor.
    /// Actualiza la disponibilidad de dicha estación remota en el caché del líder (`estaciones_info`).
    ///
    /// # Parámetros
    /// - `msg`: Mensaje TCP recibido (`INFO_ESTACION:<id>:<libres>:<ocupados>`).
    pub fn procesar_guardado_estacion_info(&mut self, msg: MensajeEntranteTcpMsg) {
        let partes = msg.0.split(':').collect::<Vec<&str>>();
        if partes.len() == 4
            && let (Ok(id_estacion), Ok(slots_libres), Ok(slots_ocupados)) = (
                partes[1].parse::<usize>(),
                partes[2].parse::<usize>(),
                partes[3].parse::<usize>(),
            )
        {
            if let Some(info) = self
                .estaciones_info
                .iter_mut()
                .find(|e| e.id == id_estacion)
            {
                info.slots_libres = slots_libres;
                info.slots_ocupados = slots_ocupados;
                info.estado = EstacionEstado::Conectada;

                println!(
                    "[Líder {}] Estado actualizado para la estación {}: {} libres, {} ocupados.",
                    self.id, id_estacion, slots_libres, slots_ocupados
                );
            } else {
                println!(
                    "[Líder {}] No se encontró la estación {} en estaciones_info.",
                    self.id, id_estacion
                );
            }
        }
    }

    /// (Solo líder) Procesa la solicitud `PREPARE_PAGO_RETIRO` de una estación, iniciando la Fase 1 del 2PC.
    /// Ejecuta la consulta al servidor procesador de pagos para inmovilizar un monto de seguridad.
    /// Responde a la estación solicitante con un `COMMIT_PAGO_RETIRO` o `ABORT_PAGO_RETIRO`.
    ///
    /// # Parámetros
    /// - `msg`: Mensaje TCP de preparación.
    /// - `ctx`: Contexto asíncrono del actor, útil para despachar tareas u otros mensajes internos.
    pub fn procesar_prepare_pago_retiro(
        &mut self,
        msg: MensajeEntranteTcpMsg,
        ctx: &mut Context<Self>,
    ) {
        let partes = msg.0.split(':').collect::<Vec<&str>>();
        if partes.len() == 10 {
            if let (
                Ok(id_estacion),
                Ok(num_slot),
                Ok(id_usuario),
                Ok(id_bicicleta),
                Ok(monto),
                Ok(cod_tarjeta),
            ) = (
                partes[1].parse::<usize>(),
                partes[2].parse::<usize>(),
                partes[3].parse::<usize>(),
                partes[4].parse::<usize>(),
                partes[5].parse::<usize>(),
                partes[8][1..4].parse::<u16>(),
            ) {
                let tarjeta_de_credito =
                    TarjetaDeCredito::new(&partes[7][2..18], cod_tarjeta, &partes[9][2..7]);

                println!(
                    "[Líder Coordinador {}] Recibido PREPARE de Estación {}. Procesando pago de pre-autorización para usuario {}...",
                    self.id, id_estacion, id_usuario
                );

                let procesador_addr = self.procesador_de_pagos;
                let id_lider = self.id;
                let mi_dir = ctx.address();

                // Obtenemos una referencia al canal del seguidor para poder responderle después
                if let Some(tx_seguidor) = self.seguidores_tx.get(&id_estacion).cloned() {
                    let tarjeta_clonada = tarjeta_de_credito.clone();
                    tokio::spawn(async move {
                        // --- FASE 1: CONSULTAR AL PROCESADOR DE PAGOS VÍA TCP ---
                        let pago_aprobado = enviar_preaturizacion_a_procesador_de_pagos(
                            id_lider,
                            id_usuario,
                            monto,
                            tarjeta_de_credito,
                            procesador_addr,
                        )
                        .await;
                        if pago_aprobado {
                            mi_dir.do_send(RegistrarAlquilerActivoMsg(
                                id_usuario,
                                id_bicicleta,
                                tarjeta_clonada,
                            ));
                        }
                        // --- FASE 2: ENVIAR RESPUESTA (COMMIT O ABORT) AL SEGUIDOR ---
                        let msg_respuesta = armar_respuesta_sobre_preautorizacion(
                            pago_aprobado,
                            id_estacion,
                            num_slot,
                            id_usuario,
                        );
                        let _ = tx_seguidor.send(msg_respuesta).await;
                    });
                } else if id_lider == id_estacion {
                    println!(
                        "[Líder Coordinador] Estación de origen es líder. Procesando localmente..."
                    );

                    let tarjeta_clonada = tarjeta_de_credito.clone();
                    tokio::spawn(async move {
                        // --- FASE 1: CONSULTAR AL PROCESADOR DE PAGOS VÍA TCP ---
                        let pago_aprobado = enviar_preaturizacion_a_procesador_de_pagos(
                            id_lider,
                            id_usuario,
                            monto,
                            tarjeta_de_credito,
                            procesador_addr,
                        )
                        .await;
                        if pago_aprobado {
                            let mi_dir_clone = mi_dir.clone();
                            mi_dir_clone.do_send(RegistrarAlquilerActivoMsg(
                                id_usuario,
                                id_bicicleta,
                                tarjeta_clonada,
                            ));
                        }
                        // --- FASE 2: PROCESAR RESPUESTA LOCALMENTE ---
                        let msg_respuesta = armar_respuesta_sobre_preautorizacion(
                            pago_aprobado,
                            id_estacion,
                            num_slot,
                            id_usuario,
                        );
                        mi_dir.do_send(MensajeEntranteTcpMsg(msg_respuesta));
                    });
                } else {
                    println!(
                        "[Líder Coordinador {}] No se encontró canal para la estación {}. No se puede procesar el PREPARE.",
                        self.id, id_estacion
                    );
                }
            } else {
                eprintln!(
                    "[Líder Coordinador {}] Error de parseo en los datos numéricos de PREPARE_PAGO_RETIRO.",
                    self.id
                );
            }
        }
    }

    /// Finaliza exitosamente el retiro de una bicicleta (Fase 2 de 2PC).
    /// Remueve el estado bloqueante del slot, asienta la entrega asíncrona a la App de Usuario y asienta todo en disco.
    ///
    /// # Parámetros
    /// - `msg`: Mensaje TCP de commit originado por el líder.
    pub fn procesar_commit_pago_retiro(&mut self, msg: MensajeEntranteTcpMsg) {
        let partes = msg.0.split(':').collect::<Vec<&str>>();
        if partes.len() == 3 {
            let num_slot = partes[1].parse::<usize>().unwrap_or(0);
            let id_usuario = partes[2].parse::<usize>().unwrap_or(0);
            let idx = num_slot - 1;

            if self.assert_numero_slot_valido(idx)
                && let EstadoSlot::PreparandoRetiro(mut bicicleta, _user_id, sender_usuario) =
                    self.slots[idx].clone()
            {
                // --- FASE 2: COMMIT ---
                self.slots[idx] = EstadoSlot::Vacio;
                bicicleta.iniciar_uso(id_usuario);

                println!(
                    "[2PC - COMMIT] ¡Pre-autorización aprobada! Bicicleta {} liberada en slot {} para usuario {}.",
                    bicicleta.id, num_slot, id_usuario
                );

                self.notificar_cambio_slots_al_lider();
                self.guardar_estado_en_disco();

                tokio::spawn(async move {
                    if let Err(e) = sender_usuario.send(bicicleta.as_bytes()).await {
                        eprintln!(
                            "[2PC - COMMIT] El usuario canceló la conexión o expiró antes de recibir la bicicleta: {:?}",
                            e
                        );
                    }
                });
            }
        }
    }

    /// Aborta el retiro de una bicicleta por fondos insuficientes u otra falla bancaria (Fase 2 de 2PC).
    /// Restaura el slot a su estado ocupado/disponible, notifica el rechazo al usuario y asienta en disco.
    ///
    /// # Parámetros
    /// - `msg`: Mensaje TCP de abort originado por el líder.
    pub fn procesar_abort_pago_retiro(&mut self, msg: MensajeEntranteTcpMsg) {
        let partes = msg.0.split(':').collect::<Vec<&str>>();
        if partes.len() == 3 {
            let num_slot = partes[1].parse::<usize>().unwrap_or(0);
            let idx = num_slot - 1;

            if self.assert_numero_slot_valido(idx)
                && let EstadoSlot::PreparandoRetiro(bicicleta, _, tx) = self.slots[idx].clone()
            {
                self.slots[idx] = EstadoSlot::Ocupado(bicicleta);

                println!(
                    "[2PC - ABORT] Pago rechazado para slot {}. La bicicleta se mantiene retenida.",
                    num_slot
                );

                self.guardar_estado_en_disco();
                tokio::spawn(async move {
                    if let Err(e) = tx.send(PagoRechazado.as_bytes()).await {
                        eprintln!(
                            "[2PC - ABORT] No se pudo notificar el rechazo del pago al usuario (canal cerrado): {:?}",
                            e
                        );
                    }
                });
            }
        }
    }

    /// (Solo líder) Despacha la solicitud de cobro final de viaje hacia el Procesador de Pagos.
    /// Remueve la bicicleta cobrada del registro de alquileres activos e invoca la comunicación TCP saliente bancaria.
    ///
    /// # Parámetros
    /// - `msg`: Mensaje de cobro proveniente de la estación que recibió la devolución de la unidad.
    pub fn procesar_cobro_viaje(&mut self, msg: MensajeEntranteTcpMsg) {
        let partes = msg.0.split(':').collect::<Vec<&str>>();
        if partes.len() == 9 {
            // Validamos y extraemos todos los campos obligatorios evitando panics por unwrap
            if let (
                Ok(id_estacion),
                Ok(id_usuario),
                Ok(id_bicicleta),
                Ok(monto_a_cobrar),
                Ok(cod_tarjeta),
            ) = (
                partes[1].parse::<usize>(),
                partes[2].parse::<usize>(),
                partes[3].parse::<usize>(),
                partes[4].parse::<usize>(),
                partes[7][1..4].parse::<u16>(),
            ) {
                let tarjeta_de_credito =
                    TarjetaDeCredito::new(&partes[6][2..18], cod_tarjeta, &partes[8][2..7]);

                if let Some(viajes) = self.alquileres_activos.get_mut(&id_usuario) {
                    let mut a_borrar = None;
                    for (i, (id_bici, _, _)) in viajes.iter().enumerate() {
                        if *id_bici == id_bicicleta {
                            a_borrar = Some(i);
                            break;
                        }
                    }
                    if let Some(i) = a_borrar {
                        viajes.remove(i);
                    }
                }

                let mut vacio = false;
                if let Some(viajes) = self.alquileres_activos.get(&id_usuario) {
                    vacio = viajes.is_empty();
                }
                if vacio {
                    self.alquileres_activos.remove(&id_usuario);
                }
                self.guardar_alquileres_en_disco();

                println!(
                    "[Líder Coordinador] Recibido COBRO_VIAJE de Estación {}. Procesando cobro de viaje para usuario {} por monto {}...",
                    id_estacion, id_usuario, monto_a_cobrar
                );

                let procesador_addr = self.procesador_de_pagos;

                tokio::spawn(async move {
                    match TcpStream::connect(procesador_addr).await {
                        Ok(mut stream) => {
                            let msg_cobro_viaje = format!(
                                "COBRO_VIAJE:{}:{}:{:?}",
                                monto_a_cobrar, MONTO_DE_SEGURIDAD, tarjeta_de_credito
                            );

                            if let Err(e) = stream.write_all(msg_cobro_viaje.as_bytes()).await {
                                eprintln!(
                                    "[Líder Coordinador] Error al enviar cobro de viaje al procesador: {}",
                                    e
                                );
                            } else {
                                let mut reader = BufReader::new(stream);
                                let mut respuesta = String::new();

                                if let Ok(bytes_leidos) = reader.read_line(&mut respuesta).await
                                    && bytes_leidos > 0
                                {
                                    if respuesta.starts_with("PAGO_VIAJE_ACEPTADO") {
                                        println!(
                                            "[Líder Coordinador] El procesador de pagos aceptó el cobro de viaje."
                                        );
                                    } else {
                                        println!(
                                            "[Líder Coordinador] El procesador de pagos rechazó el cobro de viaje."
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "[Líder Coordinador] Falló conexión con procesador de pagos en {}: {}",
                                procesador_addr, e
                            );
                        }
                    }
                });
            } else {
                eprintln!(
                    "[Líder Coordinador] Error de parseo en los datos numéricos de COBRO_VIAJE."
                );
            }
        } else {
            println!(
                "[Líder Coordinador] Mensaje COBRO_VIAJE mal formado: {}",
                msg.0
            );
        }
    }

    /// Inicializa y puebla la memoria de alquileres activos (`alquileres_activos`) basándose en el archivo `alquileres.state`.
    /// Reprograma el cálculo de multas para aquellas unidades que siguen pendientes de entrega una vez levantado el actor.
    ///
    /// # Parámetros
    /// - `ctx`: Contexto del actor, utilizado para ejecutar tareas aplazadas (`run_later`).
    pub fn recuperar_alquileres_activos(&mut self, ctx: &mut Context<Self>) {
        self.alquileres_activos.clear();
        let nombre_archivo = "src/estado_estaciones/alquileres.state";
        if let Ok(contenido) = std::fs::read_to_string(nombre_archivo) {
            println!(
                "[Líder {}] Archivo de alquileres detectado. Recuperando estado y temporizadores...",
                self.id
            );
            for linea in contenido.lines() {
                let parts: Vec<&str> = linea.split("TarjetaDeCredito").collect();
                if parts.len() == 2 {
                    let datos_viaje: Vec<&str> = parts[0].split(',').collect();
                    if datos_viaje.len() >= 3 {
                        let id_usuario = datos_viaje[0].parse::<usize>().unwrap_or(0);
                        let id_bicicleta = datos_viaje[1].parse::<usize>().unwrap_or(0);
                        let segundos_transcurridos = datos_viaje[2].parse::<u64>().unwrap_or(0);

                        let tarjeta_partes: Vec<&str> = parts[1].split(':').collect();
                        if tarjeta_partes.len() >= 4 {
                            let numero = &tarjeta_partes[1][2..18];
                            let cod = tarjeta_partes[2][1..4].parse::<u16>().unwrap_or(0);
                            let venc = &tarjeta_partes[3][2..7];
                            let tarjeta = TarjetaDeCredito::new(numero, cod, venc);

                            let nuevo_inicio =
                                Instant::now() - Duration::from_secs(segundos_transcurridos);
                            self.alquileres_activos
                                .entry(id_usuario)
                                .or_default()
                                .push((id_bicicleta, nuevo_inicio, tarjeta.clone()));

                            let espera = if segundos_transcurridos >= TIEMPO_MAX_PRE_ROBO {
                                Duration::from_secs(0)
                            } else {
                                Duration::from_secs(TIEMPO_MAX_PRE_ROBO - segundos_transcurridos)
                            };

                            ctx.run_later(espera, move |act, _ctx| {
                                chequear_y_efectuar_cobro_multa(
                                    act,
                                    id_usuario,
                                    id_bicicleta,
                                    nuevo_inicio,
                                    tarjeta,
                                );
                            });
                        }
                    }
                }
            }
        }
    }
}

impl Actor for Estacion {
    type Context = Context<Self>;
}

/// Abre una conexión TCP temporal contra el procesador de pagos para consultar si es posible inmovilizar
/// el monto de seguridad asociado al inicio de un nuevo viaje.
///
/// # Parámetros
/// - `id`: ID de la estación.
/// - `id_usuario`: ID del usuario.
/// - `monto`: Valor numérico del monto de seguridad.
/// - `tarjeta_de_credito`: Datos para imputar la operación.
/// - `procesador_de_pagos`: Dirección SocketAddr del actor procesador bancario.
///
/// # Retornos
/// - `bool`: `true` si el pago es aceptado (COMMIT), `false` si es rechazado.
async fn enviar_preaturizacion_a_procesador_de_pagos(
    id: usize,
    id_usuario: usize,
    monto: usize,
    tarjeta_de_credito: TarjetaDeCredito,
    procesador_de_pagos: SocketAddr,
) -> bool {
    let mut pago_aprobado = false;

    match TcpStream::connect(procesador_de_pagos).await {
        Ok(mut stream) => {
            let msg_prepare = format!(
                "PREPARE_PAGO_RETIRO:{}:{}:{}:{:?}",
                id, id_usuario, monto, tarjeta_de_credito,
            );

            if let Err(e) = stream.write_all(msg_prepare.as_bytes()).await {
                eprintln!(
                    "[Líder Coordinador] Error al enviar cobro al procesador: {}",
                    e
                );
            } else {
                let mut reader = BufReader::new(stream);
                let mut respuesta = String::new();

                if let Ok(bytes_leidos) = reader.read_line(&mut respuesta).await
                    && bytes_leidos > 0
                {
                    if respuesta.starts_with("COMMIT") {
                        pago_aprobado = true;
                    } else {
                        println!(
                            "[Líder Coordinador] El procesador de pagos rechazó la pre-autorización."
                        );
                    }
                }
            }
        }
        Err(e) => {
            eprintln!(
                "[Líder Coordinador] Falló conexión con procesador de pagos en {}: {}",
                procesador_de_pagos, e
            );
        }
    }

    pago_aprobado
}

/// Formatea el comando TCP de respuesta de la Fase 2 (2PC) del protocolo de alquiler hacia el nodo seguidor.
///
/// # Parámetros
/// - `pago_aprobado`: Indicador lógico de éxito.
/// - `id_estacion`: ID numérico de la estación destino.
/// - `num_slot`: Slot numérico de la bicicleta retenida.
/// - `id_usuario`: ID numérico del usuario.
///
/// # Retornos
/// - `String`: `COMMIT_PAGO_RETIRO:...` o `ABORT_PAGO_RETIRO:...` en formato plano.
fn armar_respuesta_sobre_preautorizacion(
    pago_aprobado: bool,
    id_estacion: usize,
    num_slot: usize,
    id_usuario: usize,
) -> String {
    if pago_aprobado {
        println!(
            "[Líder Coordinador] Envía COMMIT a Estación {} para Slot {}",
            id_estacion, num_slot
        );
        format!("COMMIT_PAGO_RETIRO:{}:{}", num_slot, id_usuario)
    } else {
        println!(
            "[Líder Coordinador] Envía ABORT a Estación {} para Slot {}",
            id_estacion, num_slot
        );
        format!("ABORT_PAGO_RETIRO:{}:{}", num_slot, id_usuario)
    }
}

/// Función invocada asincrónicamente por temporizador para controlar viajes excedidos en tiempo (`TIEMPO_MAX_PRE_ROBO`).
/// Si el sistema comprueba que no ha habido devolución para el rodado, envía la orden al banco para cobrar una multa.
///
/// # Parámetros
/// - `act`: Instancia mutable del propio actor `Estacion`.
/// - `id_usuario`: Usuario responsable del alquiler a ser evaluado.
/// - `id_bicicleta`: Rodado asociado al viaje evaluado.
/// - `instante_inicio`: Momento absoluto registrado cuando se inició el viaje.
/// - `tarjeta`: Credencial bancaria donde recaerá la imputación punitoria.
pub fn chequear_y_efectuar_cobro_multa(
    act: &mut Estacion,
    id_usuario: usize,
    id_bicicleta: usize,
    instante_inicio: Instant,
    tarjeta: TarjetaDeCredito,
) {
    if act.lider_actual != Some(act.id) {
        // Si ya no es líder, la multa no se procesa.
        return;
    }

    let mut multar = false;
    let mut vacio = false;

    if let Some(viajes) = act.alquileres_activos.get_mut(&id_usuario) {
        if let Some(pos) = viajes
            .iter()
            .position(|(bici_id, inicio, _)| *bici_id == id_bicicleta && *inicio == instante_inicio)
        {
            viajes.remove(pos);
            multar = true;
        }
        vacio = viajes.is_empty();
    }

    if vacio {
        act.alquileres_activos.remove(&id_usuario);
    }

    // Si la bicicleta seguía alquilada después del timeout, se efectúa la multa.
    if multar {
        println!(
            "[Líder {}] ¡Multa! Bicicleta {} robada por usuario {} al exceder el límite.",
            act.id, id_bicicleta, id_usuario
        );

        let procesador_addr = act.procesador_de_pagos;
        let tarjeta_clonada = tarjeta;
        tokio::spawn(async move {
            if let Ok(mut stream) = TcpStream::connect(procesador_addr).await {
                let msg_multa = format!("COBRO_MULTA:{}:{:?}\n", id_usuario, tarjeta_clonada);
                let _ = stream.write_all(msg_multa.as_bytes()).await;
            }
        });

        act.guardar_alquileres_en_disco();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Función auxiliar para instanciar rápidamente una estación y probar sus métodos internos.
    fn crear_estacion_dummy(id: usize, slots: Vec<EstadoSlot>) -> Estacion {
        Estacion {
            id,
            nombre: "Estación Test".to_string(),
            slots,
            coordenadas: Coordenadas::new(0, 0),
            conectado: true,
            tx_tcp: None,
            otras_estaciones: HashSet::new(),
            lider_actual: Some(id),
            procesador_de_pagos: "127.0.0.1:8080".parse().unwrap(),
            estaciones_info: vec![
                EstacionInfo {
                    id,
                    slots_libres: 0,
                    slots_ocupados: 0,
                    estado: EstacionEstado::Conectada,
                },
                EstacionInfo {
                    id: 2,
                    slots_libres: 0,
                    slots_ocupados: 0,
                    estado: EstacionEstado::Incierto,
                },
            ],
            ring_eleccion: None,
            servidor_tcp_iniciado: false,
            seguidores_tx: HashMap::new(),
            alquileres_activos: HashMap::new(),
            pagos_pendientes: VecDeque::new(),
        }
    }

    #[test]
    fn test01_assert_numero_slot_valido() {
        let estacion = crear_estacion_dummy(1, vec![EstadoSlot::Vacio, EstadoSlot::Vacio]);
        assert!(
            estacion.assert_numero_slot_valido(0),
            "El slot 0 debería ser válido"
        );
        assert!(
            estacion.assert_numero_slot_valido(1),
            "El slot 1 debería ser válido"
        );
        assert!(
            !estacion.assert_numero_slot_valido(2),
            "El slot 2 debería ser inválido"
        );
    }

    #[test]
    fn test02_estado_slot_eq() {
        let bici1 = Bicicleta::new(10, EstadoBicicleta::Disponible);
        let bici2 = Bicicleta::new(10, EstadoBicicleta::Disponible);
        let bici3 = Bicicleta::new(20, EstadoBicicleta::Disponible);

        assert_eq!(EstadoSlot::Vacio, EstadoSlot::Vacio);
        assert_eq!(
            EstadoSlot::Ocupado(bici1.clone()),
            EstadoSlot::Ocupado(bici2.clone())
        );
        assert_ne!(
            EstadoSlot::Ocupado(bici1.clone()),
            EstadoSlot::Ocupado(bici3.clone())
        );
        assert_ne!(EstadoSlot::Vacio, EstadoSlot::Ocupado(bici1.clone()));
    }

    #[test]
    fn test03_notificar_cambio_slots_al_lider_siendo_lider() {
        let mut estacion = crear_estacion_dummy(
            1,
            vec![
                EstadoSlot::Vacio,
                EstadoSlot::Ocupado(Bicicleta::new(1, EstadoBicicleta::Disponible)),
            ],
        );
        estacion.notificar_cambio_slots_al_lider();

        let mi_info = estacion.estaciones_info.iter().find(|e| e.id == 1).unwrap();
        assert_eq!(mi_info.slots_libres, 1);
        assert_eq!(mi_info.slots_ocupados, 1);
        assert!(matches!(mi_info.estado, EstacionEstado::Conectada));
    }

    #[test]
    fn test04_procesar_guardado_estacion_info() {
        let mut estacion = crear_estacion_dummy(1, vec![]);
        let msg = MensajeEntranteTcpMsg("INFO_ESTACION:2:5:15".to_string());

        estacion.procesar_guardado_estacion_info(msg);

        let info_2 = estacion.estaciones_info.iter().find(|e| e.id == 2).unwrap();
        assert_eq!(info_2.slots_libres, 5);
        assert_eq!(info_2.slots_ocupados, 15);
        assert!(matches!(info_2.estado, EstacionEstado::Conectada));
    }
}
