use super::actor::{Estacion, EstadoSlot};
use crate::actor::chequear_y_efectuar_cobro_multa;
use crate::eleccion_de_lider::{EleccionLider, conectar_con_lider_tcp, iniciar_servidor_tcp_lider};
use crate::{COSTO_POR_SEGUNDO, MONTO_DE_SEGURIDAD};
use actix::{AsyncContext, Context, Handler, Message, MessageResult};
use std::thread::yield_now;
use std::time::{Duration, Instant};
use tokio::spawn;
use tokio::sync::mpsc;
use tp::constantes::TIEMPO_MAX_PRE_ROBO;
use tp::msjs_app_usuario_estacion::{DevolverBicicleta, EnviarEstado, PedirBicicleta};
use tp::msjs_app_usuario_estacion_lider::EstacionEstado::Incierto;
use tp::msjs_app_usuario_estacion_lider::{EstacionInfo, EstacionesPedidas};
use tp::objetos_bancarios::TarjetaDeCredito;

/// Wrapper para poder tratar el mensaje de pedido de bicicleta como un mensaje de Actix,
/// ya que PedirBicicleta no se encuentra definido en este módulo.
///
/// # Atributos
/// - `0`: Solicitud `PedirBicicleta` recibida por TCP.
/// - `1`: Canal `mpsc::Sender` utilizado para responder al cliente de manera asíncrona.
pub struct PedirBicicletaMsg(pub PedirBicicleta, pub mpsc::Sender<Vec<u8>>);

impl Message for PedirBicicletaMsg {
    type Result = Option<bool>;
}

impl Handler<PedirBicicletaMsg> for Estacion {
    type Result = Option<bool>;

    /// Procesa el pedido de una bicicleta, verificando que el slot solicitado esté ocupado y enviando la información de la bicicleta al usuario.
    ///
    /// # Retornos
    /// - `Some(true)`: Si se aprobó temporalmente y se procesa el retiro (inicia pre-autorización).
    /// - `Some(false)`: Si ya se está procesando un retiro en ese slot.
    /// - `None`: Si el slot está vacío o el número de slot es inválido.
    fn handle(&mut self, msg: PedirBicicletaMsg, ctx: &mut Context<Self>) -> Self::Result {
        let pedido = msg.0;
        if !self.assert_numero_slot_valido(pedido.numero_slot as usize - 1) {
            return None;
        }

        match &self.slots[pedido.numero_slot as usize - 1] {
            EstadoSlot::Vacio => {
                println!(
                    "El slot {} está vacío, no se puede retirar una bicicleta.",
                    pedido.numero_slot
                );
                None
            }
            EstadoSlot::PreparandoRetiro(..) => {
                println!(
                    "Se está procesando una pre-autorización para la bicicleta en el slot {}, la misma no puede ser retirada por el momento.",
                    pedido.numero_slot
                );
                Some(false)
            }
            EstadoSlot::Ocupado(bicicleta) => {
                let mut bici_clonada = bicicleta.clone();

                println!(
                    "[2PC - Prepare] Solicitando pre-autorización al líder para usuario {} en slot {}.",
                    pedido.id, pedido.numero_slot
                );

                let msg_prepare = format!(
                    "PREPARE_PAGO_RETIRO:{}:{}:{}:{}:{}:{:?}",
                    self.id,
                    pedido.numero_slot,
                    pedido.id,
                    bici_clonada.id(),
                    MONTO_DE_SEGURIDAD,
                    pedido.tarjeta_de_credito,
                );

                if !self.conectado {
                    println!(
                        "[Estación {}] Aprobando alquiler localmente, la pre-autorización se efectuará cuando vuelva la conexión.",
                        self.id
                    );
                    bici_clonada.iniciar_uso(pedido.id);
                    self.slots[pedido.numero_slot as usize - 1] = EstadoSlot::Vacio;
                    self.pagos_pendientes.push_back(msg_prepare);
                    self.guardar_estado_en_disco();

                    let tx = msg.1;
                    tokio::spawn(async move {
                        let _ = tx.send(bici_clonada.as_bytes()).await;
                    });
                    return Some(true);
                }

                if self.lider_actual == Some(self.id) {
                    ctx.address().do_send(MensajeEntranteTcpMsg(msg_prepare));
                } else if let Some(ref tx) = self.tx_tcp {
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        let _ = tx_clone.send(msg_prepare).await;
                    });
                }

                self.slots[pedido.numero_slot as usize - 1] =
                    EstadoSlot::PreparandoRetiro(bici_clonada, pedido.id, msg.1);
                self.guardar_estado_en_disco();
                Some(true) // Se está procesando la pre-autorización.
            }
        }
    }
}

/// Mensaje interno para registrar que un usuario inició de forma efectiva el alquiler de una bicicleta.
///
/// # Atributos
/// - `0`: ID del usuario que retira la bicicleta.
/// - `1`: ID de la bicicleta alquilada.
/// - `2`: Objeto `TarjetaDeCredito` del usuario, utilizado en caso de tener que efectuar el cobro de una multa.
pub struct RegistrarAlquilerActivoMsg(pub usize, pub usize, pub TarjetaDeCredito);

impl Message for RegistrarAlquilerActivoMsg {
    type Result = ();
}

impl Handler<RegistrarAlquilerActivoMsg> for Estacion {
    type Result = ();

    /// Guarda en el estado interno (memoria y disco) el nuevo alquiler iniciado con su respectivo tiempo de inicio.
    /// Además, programa un chequeo asíncrono para ejecutarse una vez alcanzado el tiempo límite
    /// (`TIEMPO_MAX_PRE_ROBO`) a fin de aplicar multas si la bicicleta no fue devuelta en tiempo.
    ///
    /// # Retornos
    /// - `()`: No retorna ningún valor.
    fn handle(&mut self, msg: RegistrarAlquilerActivoMsg, ctx: &mut Context<Self>) {
        let id_usuario = msg.0;
        let id_bicicleta = msg.1;
        let tarjeta = msg.2.clone();

        let instante_inicio = Instant::now();
        self.alquileres_activos
            .entry(id_usuario)
            .or_default()
            .push((id_bicicleta, instante_inicio, msg.2));

        self.guardar_alquileres_en_disco();

        // Tarea que se despertará para verificar si es necesario aplicar la multa.
        ctx.run_later(
            Duration::from_secs(TIEMPO_MAX_PRE_ROBO),
            move |act, _ctx| {
                chequear_y_efectuar_cobro_multa(
                    act,
                    id_usuario,
                    id_bicicleta,
                    instante_inicio,
                    tarjeta,
                );
            },
        );
    }
}

/// Wrapper para poder tratar el mensaje de devolución de bicicleta como un mensaje de Actix,
/// ya que DevolverBicicleta no se encuentra definido en este módulo.
///
/// # Atributos
/// - `0`: Solicitud `DevolverBicicleta` recibida por TCP.
pub struct DevolverBicicletaMsg(pub DevolverBicicleta);

impl Message for DevolverBicicletaMsg {
    type Result = Option<usize>;
}

impl Handler<DevolverBicicletaMsg> for Estacion {
    type Result = Option<usize>;

    /// Procesa la devolución de una bicicleta, actualizando el estado del slot correspondiente y agregando el pago a procesar.
    ///
    /// # Retornos
    /// - `Some(usize)`: El monto cobrado en formato numérico si la operación fue exitosa.
    /// - `None`: Si el slot de destino ya estaba ocupado, era inválido o la bicicleta no estaba marcada en uso.
    fn handle(&mut self, msg: DevolverBicicletaMsg, ctx: &mut Context<Self>) -> Self::Result {
        let pedido = msg.0;
        if !self.assert_numero_slot_valido(pedido.numero_slot as usize - 1) {
            return None;
        }

        if self.slots[pedido.numero_slot as usize - 1] != EstadoSlot::Vacio {
            println!(
                "El slot {} ya está ocupado, no se puede devolver la bicicleta aquí.",
                pedido.numero_slot
            );
            return None;
        }

        let mut bicicleta = pedido.bicicleta;
        let inicio_uso = match bicicleta.disponibilizar() {
            Some(instante) => instante,
            None => {
                eprintln!(
                    "[Error] La bicicleta {} no se encontraba en uso.",
                    bicicleta.id
                );
                return None;
            }
        };
        let segundos_uso = inicio_uso.elapsed().as_secs();
        let monto_a_cobrar = segundos_uso as usize * COSTO_POR_SEGUNDO;

        println!(
            "Bicicleta con ID {} devuelta en slot {} después de {} segundos de uso. Monto a cobrar: {}.",
            bicicleta.id, pedido.numero_slot, segundos_uso, monto_a_cobrar
        );

        let msj_cobro_viaje = format!(
            "COBRO_VIAJE:{}:{}:{}:{}:{:?}",
            self.id, pedido.id, bicicleta.id, monto_a_cobrar, pedido.tarjeta_de_credito,
        );

        if !self.conectado {
            println!(
                "[Estación {}] Guardando pago de devolución en pendientes, no es posible procesarlo en este momento.",
                self.id
            );
            self.pagos_pendientes.push_back(msj_cobro_viaje);
        } else if self.lider_actual == Some(self.id) {
            ctx.address()
                .do_send(MensajeEntranteTcpMsg(msj_cobro_viaje));
        } else if let Some(ref tx) = self.tx_tcp {
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let _ = tx_clone.send(msj_cobro_viaje).await;
            });
        } else {
            self.pagos_pendientes.push_back(msj_cobro_viaje);
        }

        self.slots[pedido.numero_slot as usize - 1] = EstadoSlot::Ocupado(bicicleta); // Colocar la bicicleta en el slot.
        self.notificar_cambio_slots_al_lider();
        self.guardar_estado_en_disco();
        Some(monto_a_cobrar)
    }
}

/// Mensaje utilizado para solicitar el estado general de la estación.
/// Sirve para consultar cuáles slots se encuentran libres y cuáles ocupados.
pub struct SolicitarEstadoMsg;

impl Message for SolicitarEstadoMsg {
    type Result = EnviarEstado;
}

impl Handler<SolicitarEstadoMsg> for Estacion {
    type Result = MessageResult<SolicitarEstadoMsg>;

    /// Procesa la solicitud de estado de la estación, devolviendo un vector con el estado de cada slot (true si está ocupado, false si está vacío).
    ///
    /// # Retornos
    /// - `MessageResult<SolicitarEstadoMsg>`: Respuesta que empaqueta `EnviarEstado`, conteniendo los arreglos de índices libres y ocupados.
    fn handle(&mut self, _msg: SolicitarEstadoMsg, _ctx: &mut Context<Self>) -> Self::Result {
        let mut slots_libres = Vec::new();
        let mut slots_ocupados = Vec::new();

        for (slot, estado) in self.slots.iter().enumerate() {
            if matches!(estado, EstadoSlot::Vacio) {
                slots_libres.push(slot + 1);
            } else {
                slots_ocupados.push(slot + 1);
            }
        }

        MessageResult(EnviarEstado {
            slots_libres,
            slots_ocupados,
        })
    }
}

/// Mensaje interno para notificar a la estación acerca del nuevo líder del sistema (coordinador).
///
/// # Atributos
/// - `0`: ID numérico de la estación que asumió el rol de líder.
pub struct CambiarLiderMsg(pub usize);

impl Message for CambiarLiderMsg {
    type Result = ();
}

impl Handler<CambiarLiderMsg> for Estacion {
    type Result = ();

    /// Procesa el cambio de líder en el sistema actualizando su estado interno.
    /// Si la propia estación asume como líder, inicializa las tablas de estado en base a sus pares,
    /// restablece alquileres pendientes, y levanta su servidor TCP de sincronización.
    /// Si se le notifica que otro nodo es líder, inicia la conexión TCP en rol de seguidor hacia él.
    ///
    /// # Retornos
    /// - `()`: No retorna ningún valor.
    fn handle(&mut self, msg: CambiarLiderMsg, ctx: &mut Context<Self>) -> Self::Result {
        if !self.conectado {
            return;
        }

        let nuevo_lider = msg.0;
        if self.lider_actual == Some(nuevo_lider) {
            return;
        }
        if nuevo_lider == self.id && self.lider_actual.is_some() {
            println!(
                "[Estación {}] Ignorando petición de autoproclamarse líder. Ya existe el líder: {:?}",
                self.id, self.lider_actual
            );
            return;
        }
        self.lider_actual = Some(nuevo_lider);
        println!(
            "[Estación {}] Líder actualizado mediante Ring a: {}",
            self.id, nuevo_lider
        );
        let id_propio = self.id;
        let estacion_clone = ctx.address();
        self.tx_tcp = None;

        if nuevo_lider == id_propio {
            self.estaciones_info.clear();
            self.seguidores_tx.clear();

            let slots_libres = self
                .slots
                .iter()
                .filter(|s| matches!(s, EstadoSlot::Vacio))
                .count();
            let slots_ocupados = self.slots.len() - slots_libres;

            self.estaciones_info.push(EstacionInfo {
                id: self.id,
                slots_libres,
                slots_ocupados,
                estado: tp::msjs_app_usuario_estacion_lider::EstacionEstado::Conectada,
            });

            for id_otra in &self.otras_estaciones {
                self.estaciones_info.push(EstacionInfo {
                    id: *id_otra,
                    slots_libres: 0,
                    slots_ocupados: 0,
                    estado: tp::msjs_app_usuario_estacion_lider::EstacionEstado::Incierto,
                });
            }

            println!(
                "[Estación {}] He asumido como líder. Se inicializó estaciones_info con {} estaciones.",
                self.id,
                self.estaciones_info.len()
            );

            self.recuperar_alquileres_activos(ctx);

            let pagos: Vec<String> = self.pagos_pendientes.drain(..).collect();
            for pago in pagos {
                println!(
                    "[Estación {}] Procesando pago pendiente localmente (ahora soy líder).",
                    self.id
                );
                ctx.address().do_send(MensajeEntranteTcpMsg(pago));
            }

            if !self.servidor_tcp_iniciado {
                self.servidor_tcp_iniciado = true;
                tokio::spawn(async move {
                    iniciar_servidor_tcp_lider(id_propio, estacion_clone).await;
                });
            } else {
                self.servidor_tcp_iniciado = false;
                println!(
                    "[Estación {}] El servidor TCP de sincronización ya estaba activo. Reutilizándolo.",
                    self.id
                );
            }
        } else {
            tokio::spawn(async move {
                conectar_con_lider_tcp(id_propio, nuevo_lider, estacion_clone).await;
            });
        }
    }
}

/// Mensaje que consulta al actor la identidad (`id`) del líder actual que controla el sistema.
pub struct ObtenerLiderActualMsg;

impl Message for ObtenerLiderActualMsg {
    type Result = Option<usize>;
}

impl Handler<ObtenerLiderActualMsg> for Estacion {
    type Result = Option<usize>;

    /// Devuelve el ID del lider actual.
    ///
    /// # Retornos
    /// - `Some(usize)` con el ID del líder.
    /// - `None` si en el momento actual no se conoce el líder.
    fn handle(&mut self, _msg: ObtenerLiderActualMsg, _ctx: &mut Context<Self>) -> Self::Result {
        self.lider_actual
    }
}

/// Mensaje para solicitar al líder el estado actualizado de múltiples estaciones en simultáneo.
///
/// # Atributos
/// - `estaciones`: Arreglo de IDs de estaciones de las cuales se requiere conocer su estado (slots y conectividad).
pub struct ObtenerVisualizacionEstacionesMsg {
    pub estaciones: Vec<usize>,
}

impl Message for ObtenerVisualizacionEstacionesMsg {
    type Result = EstacionesPedidas;
}

impl Handler<ObtenerVisualizacionEstacionesMsg> for Estacion {
    type Result = MessageResult<ObtenerVisualizacionEstacionesMsg>;

    /// Consulta el estado interno (caché) mantenido por el líder respecto de otras estaciones
    /// y devuelve solo la información correspondiente a los IDs que fueron solicitados.
    ///
    /// # Retornos
    /// - `MessageResult<ObtenerVisualizacionEstacionesMsg>`: Estructura consolidada con la información requerida.
    fn handle(
        &mut self,
        msg: ObtenerVisualizacionEstacionesMsg,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        // SI UNA ESTACIÓN NO ES LIDER DEBERÍA RETORNAR DE UNA SIN HACER NADA
        let mut estaciones_info = Vec::new();
        for id in msg.estaciones {
            if let Some(estacion) = self.estaciones_info.iter().find(|e| e.id == id) {
                estaciones_info.push(estacion.clone());
            }
        }
        MessageResult(EstacionesPedidas {
            estaciones: estaciones_info,
        })
    }
}

// Comunicación TCP con líder

/// Mensaje que notifica la creación exitosa de un canal de comunicación TCP saliente (generalmente hacia el líder).
///
/// # Atributos
/// - `0`: Canal `mpsc::Sender<String>` asíncrono para escribir en el flujo TCP.
pub struct NuevaConexionTcpMsg(pub mpsc::Sender<String>);

impl Message for NuevaConexionTcpMsg {
    type Result = ();
}

impl Handler<NuevaConexionTcpMsg> for Estacion {
    type Result = ();

    /// Asocia el canal TCP provisto a la estación actual si ésta actúa como seguidor.
    /// Además, despacha de manera inicial un resumen de estado de sus slots y los pagos que habían
    /// quedado encolados/pendientes mientras operaba fuera de línea.
    fn handle(&mut self, msg: NuevaConexionTcpMsg, _ctx: &mut Context<Self>) {
        let tx = msg.0;
        if self.lider_actual == Some(self.id) {
        } else {
            self.tx_tcp = Some(tx.clone());
            enviar_estado_a_lider(&self.slots, self.id, tx.clone());
            enviar_pagos_pendientes_a_lider(self.pagos_pendientes.drain(..).collect(), self.id, tx);
        }
    }
}

/// Calcula la proporción de slots libres y ocupados y despacha esa información inicial
/// al coordinador líder mediante un mensaje prefijado con `INFO_ESTACION`.
///
/// # Parámetros
/// - `slots`: Referencia al arreglo de slots locales para contabilizar disponibilidades.
/// - `id_estacion`: ID numérico local para poder identificarse ante el líder.
/// - `tx`: Canal de escritura asíncrona hacia la conexión TCP del líder.
fn enviar_estado_a_lider(slots: &[EstadoSlot], id_estacion: usize, tx: mpsc::Sender<String>) {
    let slots_libres = slots
        .iter()
        .filter(|s| matches!(s, EstadoSlot::Vacio))
        .count();
    let slots_ocupados = slots.len() - slots_libres;

    let mensaje_estado = format!(
        "INFO_ESTACION:{}:{}:{}",
        id_estacion, slots_libres, slots_ocupados
    );
    println!(
        "[Estación {}] ¡Conexión TCP lista! Enviando estado al líder: {} libres, {} ocupados.",
        id_estacion, slots_libres, slots_ocupados
    );
    tokio::spawn(async move {
        if let Err(e) = tx.send(mensaje_estado).await {
            eprintln!(
                "[Estación {}] Error al enviar info inicial al líder: {}",
                id_estacion, e
            );
        }
    });
}

/// Flujo orquestador para el envío de pagos pendientes de una estación hacia el líder al reconectarse.
/// Delega primero el envío de preautorizaciones (Prepare) y luego el de cobros directos.
///
/// # Parámetros
/// - `pagos`: Arreglo de strings serializados con los comandos de pago acumulados.
/// - `id_estacion`: ID numérico de la estación base.
/// - `tx`: Canal de envío asíncrono hacia la conexión del líder.
fn enviar_pagos_pendientes_a_lider(
    pagos: Vec<String>,
    id_estacion: usize,
    tx: mpsc::Sender<String>,
) {
    enviar_preaturizacion_a_lider(pagos.clone(), id_estacion, tx.clone());
    yield_now();
    enviar_cobro_viaje_a_lider(pagos, id_estacion, tx);
}

/// Filtra y envía únicamente los comandos de tipo "PREPARE" desde una lista de pagos acumulados.
///
/// # Parámetros
/// - `pagos`: Lista completa de comandos pendientes.
/// - `id_estacion`: ID numérico de la estación remitente.
/// - `tx`: Canal TCP del líder a donde inyectar el mensaje.
fn enviar_preaturizacion_a_lider(pagos: Vec<String>, id_estacion: usize, tx: mpsc::Sender<String>) {
    tokio::spawn(async move {
        for pago in pagos {
            if pago.starts_with("PREPARE:") {
                println!(
                    "[Estación {}] Enviando pre-autorización al líder: {}",
                    id_estacion, pago
                );
                let _ = tx.send(pago).await;
            }
        }
    });
}

/// Filtra y envía únicamente los comandos de tipo "COBRO_VIAJE" de la cola de pagos locales hacia el líder.
///
/// # Parámetros
/// - `pagos`: Lista de comandos de pagos en formato cadena (string).
/// - `id_estacion`: ID numérico local.
/// - `tx`: Canal TCP del líder.
fn enviar_cobro_viaje_a_lider(pagos: Vec<String>, id_estacion: usize, tx: mpsc::Sender<String>) {
    tokio::spawn(async move {
        for pago in pagos {
            if pago.starts_with("COBRO_VIAJE:") {
                println!(
                    "[Estación {}] Enviando pago de viaje al líder: {}",
                    id_estacion, pago
                );
                let _ = tx.send(pago).await;
            }
        }
    });
}

/// Envuelve un string de texto puro que ha sido recibido a través de la comunicación TCP.
///
/// # Atributos
/// - `0`: Contenido de la línea de texto deserializada proveniente de la red.
pub struct MensajeEntranteTcpMsg(pub String);

impl Message for MensajeEntranteTcpMsg {
    type Result = ();
}

impl Handler<MensajeEntranteTcpMsg> for Estacion {
    type Result = ();

    /// Actúa como ruteador (despachador) para comandos basados en texto que llegan por TCP.
    /// Si la estación está desconectada los ignora. Si está conectada, re-dirige el flujo según
    /// el prefijo (`INFO_ESTACION`, `PREPARE_PAGO_RETIRO`, `COMMIT_PAGO_RETIRO`, `ABORT_PAGO_RETIRO`, etc.)
    /// a sus funciones internas correspondientes.
    fn handle(&mut self, msg: MensajeEntranteTcpMsg, ctx: &mut Context<Self>) {
        if !self.conectado {
            return;
        }

        println!("Mensaje recibido por TCP: {}", msg.0);

        if msg.0.starts_with("INFO_ESTACION:") {
            self.procesar_guardado_estacion_info(msg);
        } else if msg.0.starts_with("PREPARE_PAGO_RETIRO:") {
            self.procesar_prepare_pago_retiro(msg, ctx);
        } else if msg.0.starts_with("COMMIT_PAGO_RETIRO:") {
            self.procesar_commit_pago_retiro(msg);
        } else if msg.0.starts_with("ABORT_PAGO_RETIRO:") {
            self.procesar_abort_pago_retiro(msg);
        } else if msg.0.starts_with("COBRO_VIAJE:") {
            self.procesar_cobro_viaje(msg);
        } else {
            println!("Mensaje TCP no reconocido: {}", msg.0);
        }
    }
}

/// Mensaje interno asíncrono para notificar al actor Estacion que la conexión TCP de su líder se ha cortado (EOF).
pub struct LiderCaidoMsg;

impl Message for LiderCaidoMsg {
    type Result = ();
}

impl Handler<LiderCaidoMsg> for Estacion {
    type Result = ();

    /// Maneja el suceso de una desconexión crítica del líder.
    /// Limpia las credenciales del líder actual (`tx_tcp`, `lider_actual`) y si la estación no ha cortado
    /// artificialmente su conexión, ordena al subsistema de anillo UDP que inicie un nuevo protocolo
    /// de consenso para escoger líder.
    fn handle(&mut self, _msg: LiderCaidoMsg, _ctx: &mut Context<Self>) -> Self::Result {
        if !self.conectado {
            return;
        }

        // Evitamos bucles si ya sabíamos que se cayó
        if self.lider_actual.is_none() {
            return;
        }

        println!(
            "[Estación {}] ¡Se detectó la caída del líder por TCP! Disparando nueva elección...",
            self.id
        );
        self.lider_actual = None;
        self.tx_tcp = None;

        if let Some(ref ring) = self.ring_eleccion {
            println!(
                "[Estación {}] Iniciando protocolo de consenso en el Ring UDP...",
                self.id
            );
            ring.disparar_eleccion_sincrono();
        } else {
            eprintln!(
                "[Estación {}] Error: Se detectó líder caído pero el Ring de elección no está configurado.",
                self.id
            );
        }
    }
}

/// Mensaje inyector para proporcionarle a la estación de Actix una referencia al controlador
/// de elecciones por UDP (Ring).
///
/// # Atributos
/// - `0`: Instancia generada y en ejecución de `EleccionLider`.
pub struct ConfigurarRingMsg(pub EleccionLider);

impl Message for ConfigurarRingMsg {
    type Result = ();
}

impl Handler<ConfigurarRingMsg> for Estacion {
    type Result = ();

    /// Aloja y preserva la entidad del anillo de consenso dentro del estado local de la estación.
    fn handle(&mut self, msg: ConfigurarRingMsg, _ctx: &mut Context<Self>) -> Self::Result {
        self.ring_eleccion = Some(msg.0);
        println!(
            "[Estación {}] Módulo de elección de líder vinculado correctamente al Actor.",
            self.id
        );
    }
}

/// Mensaje que notifica al coordinador líder que uno de sus nodos clientes (seguidores) cerró la conexión TCP.
///
/// # Atributos
/// - `0`: ID numérico del seguidor que acaba de caer.
pub struct SeguidorCaidoMsg(pub usize);

impl Message for SeguidorCaidoMsg {
    type Result = ();
}

impl Handler<SeguidorCaidoMsg> for Estacion {
    type Result = ();

    /// Al remover la conexión de un seguidor, borra su canal de comunicación registrado (`seguidores_tx`)
    /// y marca el estado de red de la estación específica como "Incierto" (`EstacionEstado::Incierto`)
    /// para que el resto del sistema lo tenga en cuenta en futuras consultas.
    fn handle(&mut self, msg: SeguidorCaidoMsg, _ctx: &mut Self::Context) -> Self::Result {
        let id_caido = msg.0;

        for estacion in &mut self.estaciones_info {
            if estacion.id == id_caido {
                self.seguidores_tx.remove(&id_caido);
                estacion.estado = Incierto;
            }
        }
    }
}

/// Registra ante un nodo líder los canales bidireccionales con un nuevo nodo seguidor que acaba de conectar.
pub struct RegistrarSeguidorMsg {
    /// ID numérico único declarado por el seguidor en su fase de `HANDSHAKE`.
    pub id_seguidor: usize,
    /// Canal para inyectarle mensajes en texto plano y que viajen hacia el seguidor mediante TCP.
    pub tx: mpsc::Sender<String>,
}

impl Message for RegistrarSeguidorMsg {
    type Result = ();
}

impl Handler<RegistrarSeguidorMsg> for Estacion {
    type Result = ();

    /// Comprueba que este nodo siga siendo el líder. Si lo es, guarda el canal en `seguidores_tx`
    /// bajo la clave del `id_seguidor` para futuros comandos en cascada (2PC o Sync).
    fn handle(&mut self, msg: RegistrarSeguidorMsg, _ctx: &mut Context<Self>) {
        if self.lider_actual == Some(self.id) {
            self.seguidores_tx.insert(msg.id_seguidor, msg.tx);
            println!(
                "[Líder {}] Canal TCP del seguidor {} registrado exitosamente.",
                self.id, msg.id_seguidor
            );
        }
    }
}

/// Mensaje externo, habitualmente originado por input interactivo (CLI), para forzar (simular)
/// el apagado o el restablecimiento lógico del subsistema de red de esta estación.
pub struct CambiarEstadoConectividad;

impl Message for CambiarEstadoConectividad {
    type Result = ();
}

impl Handler<CambiarEstadoConectividad> for Estacion {
    type Result = ();

    /// Alterna el booleano `conectado` del estado. Al cortarse la conexión simula la caída: notifica cierre
    /// de red en los canales UDP (`ring_eleccion`) e inyecta "DESCONECTAR" en canales TCP abiertos hacia
    /// seguidores o líder. Al restablecer conexión, instruye al Ring UDP borrar su caché de líderes y
    /// dispara en el acto una nueva elección sincrónica para volver a unirse al esquema.
    fn handle(&mut self, _: CambiarEstadoConectividad, _: &mut Self::Context) -> Self::Result {
        self.conectado = !self.conectado;
        if let Some(ref ring) = self.ring_eleccion {
            ring.cambiar_estado_de_conectividad(self.conectado);
        }

        if !self.conectado {
            println!("[Estación {}] Desconectada.", self.id);
            self.lider_actual = None;

            if let Some(tx) = self.tx_tcp.take() {
                spawn(async move {
                    if let Err(e) = tx.send("DESCONECTAR".to_string()).await {
                        eprintln!(
                            "[Estación] No se pudo notificar desconexión al canal del líder: {:?}",
                            e
                        );
                    }
                });
            }

            for (id_seguidor, tx) in self.seguidores_tx.drain() {
                spawn(async move {
                    if let Err(e) = tx.send("DESCONECTAR".to_string()).await {
                        eprintln!(
                            "[Estación] No se pudo notificar desconexión al seguidor {}: {:?}",
                            id_seguidor, e
                        );
                    }
                });
            }
        } else {
            println!(
                "[Estación {}] Conectada. Buscando líder y procesando pagos pendientes...",
                self.id
            );
            if let Some(ref ring) = self.ring_eleccion {
                ring.eliminar_utlimo_lider_coordinado();
                ring.disparar_eleccion_sincrono();
            }
        }
    }
}

// ---------------------------------------------------------
// SECCIÓN DE TESTS UNITARIOS
// ---------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{Estacion, EstadoSlot};
    use actix::Actor;
    use std::collections::{HashMap, HashSet, VecDeque};
    use tokio::sync::mpsc;
    use tp::coordenadas::Coordenadas;
    use tp::msjs_app_usuario_estacion::{
        Bicicleta, DevolverBicicleta, EstadoBicicleta, PedirBicicleta,
    };
    use tp::objetos_bancarios::TarjetaDeCredito;

    // Función constructora auxiliar (Helper) para inicializar una Estacion controlada y limpia para cada test
    fn crear_estacion_de_prueba(id: usize, slots: Vec<EstadoSlot>) -> Estacion {
        Estacion {
            id,
            nombre: "Estacion de Prueba".to_string(),
            slots,
            coordenadas: Coordenadas::new(0, 0),
            conectado: true,
            tx_tcp: None,
            otras_estaciones: HashSet::new(),
            lider_actual: Some(1),
            procesador_de_pagos: "127.0.0.1:10000".parse().unwrap(),
            estaciones_info: Vec::new(),
            ring_eleccion: None,
            servidor_tcp_iniciado: false,
            seguidores_tx: HashMap::new(),
            alquileres_activos: HashMap::new(),
            pagos_pendientes: VecDeque::new(),
        }
    }

    #[actix::test]
    async fn test01_pedir_bicicleta_exitoso_pasa_a_preparando_retiro() {
        let bicicleta = Bicicleta::new(100, EstadoBicicleta::Disponible);
        let slots = vec![EstadoSlot::Ocupado(bicicleta)];
        let estacion = crear_estacion_de_prueba(1, slots);

        let addr = estacion.start();
        let (tx_usuario, _rx_usuario) = mpsc::channel(1);

        let pedido = PedirBicicleta {
            id: 42,
            numero_slot: 1,
            tarjeta_de_credito: TarjetaDeCredito::new("1234567890123456", 123, "12/29"),
        };

        let resultado = addr
            .send(PedirBicicletaMsg(pedido, tx_usuario))
            .await
            .unwrap();
        assert_eq!(resultado, Some(true));
    }

    #[actix::test]
    async fn test02_pedir_bicicleta_en_slot_vacio_retorna_none() {
        let slots = vec![EstadoSlot::Vacio];
        let estacion = crear_estacion_de_prueba(1, slots);
        let addr = estacion.start();
        let (tx_usuario, _rx_usuario) = mpsc::channel(1);

        let pedido = PedirBicicleta {
            id: 42,
            numero_slot: 1,
            tarjeta_de_credito: TarjetaDeCredito::new("1234567890123456", 123, "12/29"),
        };

        let resultado = addr
            .send(PedirBicicletaMsg(pedido, tx_usuario))
            .await
            .unwrap();
        assert_eq!(resultado, None);
    }

    #[actix::test]
    async fn test03_pedir_bicicleta_estacion_desconectada_retorna_true_y_encola_pago() {
        let id_bici = 100;
        let bicicleta = Bicicleta::new(id_bici, EstadoBicicleta::Disponible);
        let slots = vec![EstadoSlot::Ocupado(bicicleta)];
        let mut estacion = crear_estacion_de_prueba(1, slots);

        estacion.conectado = false;

        let addr = estacion.start();

        let (tx_usuario, mut rx_usuario) = mpsc::channel(1);

        let id_usuario = 42;
        let pedido = PedirBicicleta {
            id: id_usuario,
            numero_slot: 1,
            tarjeta_de_credito: TarjetaDeCredito::new("1234567890123456", 123, "12/29"),
        };

        let resultado = addr
            .send(PedirBicicletaMsg(pedido, tx_usuario))
            .await
            .unwrap();

        assert_eq!(resultado, Some(true)); // Se retorna Some(true) (la estación igual encola el pago).

        let bytes_recibidos = rx_usuario.recv().await;
        assert!(
            bytes_recibidos.is_some(),
            "El usuario debió recibir la bicicleta a través del canal"
        );
        let bytes = bytes_recibidos.unwrap();
        assert!(
            !bytes.is_empty(),
            "Los bytes de la bicicleta no deberían estar vacíos"
        );
    }

    #[actix::test]
    async fn test04_devolver_bicicleta_en_slot_vacio_exitoso() {
        let slots = vec![EstadoSlot::Vacio];
        let estacion = crear_estacion_de_prueba(1, slots);
        let addr = estacion.start();

        let id_usuario = 42;
        let instante_inicio = Instant::now() - Duration::from_secs(60);

        let bicicleta = Bicicleta::new(200, EstadoBicicleta::EnUso(instante_inicio, id_usuario));

        let pedido = DevolverBicicleta {
            id: id_usuario,
            numero_slot: 1,
            tarjeta_de_credito: TarjetaDeCredito::new("1234567890123456", 123, "12/29"),
            bicicleta,
        };

        let resultado = addr.send(DevolverBicicletaMsg(pedido)).await.unwrap();
        assert!(resultado.is_some());
        assert_eq!(resultado.unwrap(), 60);
    }

    #[actix::test]
    async fn test05_devolver_bicicleta_en_slot_ocupado_falla() {
        let bici_existente = Bicicleta::new(101, EstadoBicicleta::Disponible);
        let slots = vec![EstadoSlot::Ocupado(bici_existente)];
        let estacion = crear_estacion_de_prueba(1, slots);
        let addr = estacion.start();

        let bici_nueva = Bicicleta::new(200, EstadoBicicleta::Disponible);
        let pedido = DevolverBicicleta {
            id: 42,
            numero_slot: 1,
            tarjeta_de_credito: TarjetaDeCredito::new("1234567890123456", 123, "12/29"),
            bicicleta: bici_nueva,
        };

        let resultado = addr.send(DevolverBicicletaMsg(pedido)).await.unwrap();
        assert!(resultado.is_none());
    }

    #[actix::test]
    async fn test06_recepcion_mensaje_commit_2pc_despacha_bici_al_canal_del_usuario() {
        // 1. Arrange: Preparamos un slot en estado de transición (Fase 1 completada)
        let bicicleta = Bicicleta::new(300, EstadoBicicleta::Disponible);
        let (tx_usuario, mut rx_usuario) = mpsc::channel(1);

        let slots = vec![EstadoSlot::PreparandoRetiro(bicicleta, 42, tx_usuario)];
        let estacion = crear_estacion_de_prueba(1, slots);
        let addr = estacion.start();

        addr.send(MensajeEntranteTcpMsg("COMMIT_PAGO_RETIRO:1:42".to_string()))
            .await
            .unwrap();

        let bytes_recibidos: Option<Vec<u8>> = rx_usuario.recv().await;
        assert!(
            bytes_recibidos.is_some(),
            "El canal del usuario debió recibir bytes"
        );
        assert!(
            !bytes_recibidos.unwrap().is_empty(),
            "Los bytes de EntregarBicicleta no deben estar vacíos"
        );
    }

    #[actix::test]
    async fn test07_recepcion_mensaje_abort_2pc_restaura_bici_y_envia_rechazo() {
        let bicicleta = Bicicleta::new(300, EstadoBicicleta::Disponible);
        let (tx_usuario, mut rx_usuario) = mpsc::channel(1);

        let slots = vec![EstadoSlot::PreparandoRetiro(bicicleta, 42, tx_usuario)];
        let estacion = crear_estacion_de_prueba(1, slots);
        let addr = estacion.start();

        addr.send(MensajeEntranteTcpMsg("ABORT_PAGO_RETIRO:1:42".to_string()))
            .await
            .unwrap();
        let bytes_recibidos = rx_usuario.recv().await;
        assert!(bytes_recibidos.is_some());
    }

    #[actix::test]
    async fn test08_registrar_seguidor_guarda_en_mapa_si_es_lider() {
        let slots = vec![EstadoSlot::Vacio];
        let mut estacion = crear_estacion_de_prueba(5, slots);
        estacion.lider_actual = Some(5); // Yo soy el líder
        let addr = estacion.start();

        let (tx_seguidor, _rx_seguidor) = mpsc::channel(1);

        addr.send(RegistrarSeguidorMsg {
            id_seguidor: 2,
            tx: tx_seguidor,
        })
        .await
        .unwrap();
    }
}
