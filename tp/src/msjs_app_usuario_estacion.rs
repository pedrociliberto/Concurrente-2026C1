use crate::error::ErrorParseo;
use crate::objetos_bancarios::TarjetaDeCredito;
use std::time::Instant;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Describe las posibles situaciones de disponibilidad de una bicicleta dentro del sistema.
#[derive(Clone, Debug, PartialEq)]
pub enum EstadoBicicleta {
    /// La bicicleta está retenida en un slot y lista para ser alquilada.
    Disponible,
    /// La bicicleta está físicamente siendo usada por un usuario.
    EnUso(Instant, usize), // Instante en el que se comenzó a usar la bicicleta e ID del usuario que la posee.
}

/// Representa una unidad de bicicleta que puede ser quitada o liberada, por algún usuario, de uno de los slots de una estación.
#[derive(Clone, Debug, PartialEq)]
pub struct Bicicleta {
    pub id: usize,
    pub estado: EstadoBicicleta,
}

impl Bicicleta {
    /// Constructor base de una unidad Bicicleta.
    pub fn new(id: usize, estado: EstadoBicicleta) -> Self {
        Bicicleta { id, estado }
    }

    /// Devuelve el identificador de la bicicleta.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Pasa el estado de la bicicleta de `EnUso` a `Disponible`.
    /// Retorna el instante original de inicio de uso, necesario para calcular las facturaciones,
    /// o `None` si la bicicleta ya estaba libre.
    pub fn disponibilizar(&mut self) -> Option<Instant> {
        let inicio_uso = match self.estado {
            EstadoBicicleta::Disponible => None,
            EstadoBicicleta::EnUso(inicio_uso, _) => Some(inicio_uso),
        };

        self.estado = EstadoBicicleta::Disponible;
        inicio_uso
    }

    /// Actualiza el estado de la bicicleta a `EnUso`, guardando el instante de tiempo actual y el ID del usuario.
    pub fn iniciar_uso(&mut self, user_id: usize) {
        self.estado = EstadoBicicleta::EnUso(Instant::now(), user_id);
    }

    /// Serializa la estructura de la bicicleta (ID, estado y timestamps) a bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&self.id.to_be_bytes());
        match &self.estado {
            EstadoBicicleta::Disponible => {
                bytes.push(0);
            }
            EstadoBicicleta::EnUso(instant, user_id) => {
                bytes.push(1);

                let ahora_instant = Instant::now();
                let ahora_sistema = SystemTime::now();
                let transcurrido = ahora_instant.duration_since(*instant);
                let hora_alquiler = ahora_sistema
                    .checked_sub(transcurrido)
                    .unwrap_or(ahora_sistema);

                let timestamp = hora_alquiler
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                bytes.extend(&timestamp.to_be_bytes());
                bytes.extend(&user_id.to_be_bytes());
            }
        }
        bytes
    }

    /// Deserializa la estructura de la bicicleta reconstruyendo sus variables, incluso resincronizando el instante de inicio de alquiler.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.len() < 9 {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        let id = usize::from_be_bytes(
            bytes[0..8]
                .try_into()
                .map_err(|_| ErrorParseo::NoSePudoParsear)?,
        );
        let estado = match bytes[8] {
            0 => EstadoBicicleta::Disponible,
            1 => {
                if bytes.len() < 25 {
                    return Err(ErrorParseo::NoSePudoParsear);
                }
                let timestamp = u64::from_be_bytes(
                    bytes[9..17]
                        .try_into()
                        .map_err(|_| ErrorParseo::NoSePudoParsear)?,
                );
                let user_id = usize::from_be_bytes(
                    bytes[17..25]
                        .try_into()
                        .map_err(|_| ErrorParseo::NoSePudoParsear)?,
                );

                let ahora_sistema = SystemTime::now();
                let ahora_instant = Instant::now();
                let hora_alquiler = UNIX_EPOCH + Duration::from_secs(timestamp);
                let diferencia = ahora_sistema
                    .duration_since(hora_alquiler)
                    .unwrap_or_default();

                let nuevo_instant = ahora_instant
                    .checked_sub(diferencia)
                    .unwrap_or(ahora_instant);

                EstadoBicicleta::EnUso(nuevo_instant, user_id)
            }
            _ => return Err(ErrorParseo::NoSePudoParsear), // Valor de estado no reconocido.
        };
        Ok(Self { id, estado })
    }
}

// -----------------------------------------------------

// Envía AppUsuario:

const TIPO_SOLICITAR_ESTADO: u8 = 0;
const TIPO_SOLICITAR_LIDER: u8 = 1;
const BYTES_MSJ_PEDIR_BICICLETA: usize = 32;
const BYTES_MSJ_DEVOLVER_BICICLETA: usize = 41;

/// Mensaje UDP que el usuario envía para preguntar la disponibilidad de espacios en una estación específica.
#[derive(Debug)]
pub struct SolicitarEstado;

impl SolicitarEstado {
    /// Empaqueta la instrucción en bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        vec![TIPO_SOLICITAR_ESTADO]
    }

    /// Interpreta el comando a partir del flujo de bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.len() != 1 || bytes[0] != TIPO_SOLICITAR_ESTADO {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        Ok(Self)
    }
}

/// Mensaje TCP mediante el cual un usuario solicita iniciar el retiro (alquiler) de una bicicleta
/// ubicada en un slot numérico específico de la estación. Adjunta sus credenciales para la preautorización.
#[derive(Debug)]
pub struct PedirBicicleta {
    pub id: usize,
    pub numero_slot: u8, // Número de slot, entre 0 y 19.
    pub tarjeta_de_credito: TarjetaDeCredito,
}

impl PedirBicicleta {
    /// Empaqueta los datos personales y bancarios junto con la solicitud a bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&self.id.to_be_bytes());
        bytes.push(self.numero_slot);
        bytes.extend(self.tarjeta_de_credito.as_bytes());
        bytes
    }

    /// Reconstruye el objeto de alquiler inicial a partir de bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.len() != BYTES_MSJ_PEDIR_BICICLETA {
            return Err(ErrorParseo::NoSePudoParsear); // No hay suficientes bytes para un mensaje válido.
        }
        let id = usize::from_be_bytes(
            bytes[0..8]
                .try_into()
                .map_err(|_| ErrorParseo::NoSePudoParsear)?,
        );
        let numero_slot = bytes[8];
        let tarjeta_de_credito = TarjetaDeCredito::from_bytes(&bytes[9..])?;
        Ok(Self {
            id,
            numero_slot,
            tarjeta_de_credito,
        })
    }
}

/// Mensaje TCP mediante el cual el usuario solicita finalizar el viaje entregando la bicicleta en un
/// slot particular y adjuntando sus datos de pago para la facturación final.
#[derive(Debug)]
pub struct DevolverBicicleta {
    pub id: usize,
    pub numero_slot: u8, // Número de slot, entre 0 y 19.
    pub tarjeta_de_credito: TarjetaDeCredito,
    pub bicicleta: Bicicleta,
}

impl DevolverBicicleta {
    /// Serializa los datos del usuario, bicicleta y tarjeta a un vector de bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&self.id.to_be_bytes());
        bytes.push(self.numero_slot);
        bytes.extend(self.tarjeta_de_credito.as_bytes());
        bytes.extend(&self.bicicleta.as_bytes());
        bytes
    }

    /// Deserializa todos los datos necesarios para la devolución en la estación.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.len() < BYTES_MSJ_DEVOLVER_BICICLETA {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        let id = usize::from_be_bytes(
            bytes[0..8]
                .try_into()
                .map_err(|_| ErrorParseo::NoSePudoParsear)?,
        );
        let numero_slot = bytes[8];
        let tarjeta_de_credito = TarjetaDeCredito::from_bytes(&bytes[9..32])?;
        let bicicleta = Bicicleta::from_bytes(&bytes[32..])?;
        Ok(Self {
            id,
            numero_slot,
            tarjeta_de_credito,
            bicicleta,
        })
    }
}

// Envía Estacion:

const TIPO_ENTREGAR_BICI: u8 = 10;
const TIPO_NO_HAY_BICI: u8 = 11;
const TIPO_BICI_DEVUELTA_OK: u8 = 12;
const TIPO_BICI_DEVUELTA_ERR: u8 = 13;
const TIPO_ENVIAR_ESTADO: u8 = 14;
const TIPO_ENVIAR_LIDER_ACTUAL: u8 = 15;
const TIPO_SLOT_EN_PROCESO: u8 = 16;
const TIPO_PAGO_RECHAZADO: u8 = 17;

/// Respuesta TCP exitosa por parte de la estación informándole a la aplicación de usuario que su alquiler
/// fue pre-autorizado y que puede llevarse la unidad física.
pub struct EntregarBicicleta {
    pub bicicleta: Bicicleta,
}

impl EntregarBicicleta {
    /// Adjunta los datos de la bicicleta a liberar en bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![TIPO_ENTREGAR_BICI];
        bytes.extend(self.bicicleta.as_bytes());
        bytes
    }

    /// Deserializa la entrega del lado de la aplicación del usuario.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.is_empty() || bytes[0] != TIPO_ENTREGAR_BICI {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        let bicicleta = Bicicleta::from_bytes(&bytes[1..])?;
        Ok(Self { bicicleta })
    }
}

/// Respuesta TCP negativa que indica al usuario que en el slot solicitado físicamente no hay una bicicleta apta.
pub struct NoTengoBicicletaEnEseSlot {
    pub numero_slot: u8,
}

impl NoTengoBicicletaEnEseSlot {
    /// Prepara el identificador de error en bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        vec![TIPO_NO_HAY_BICI, self.numero_slot]
    }

    /// Deserializa el error.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.len() < 2 || bytes[0] != TIPO_NO_HAY_BICI {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        Ok(Self {
            numero_slot: bytes[1],
        })
    }
}

/// Respuesta TCP negativa indicando que ya existe un proceso de alquiler en curso sobre esa misma bicicleta.
pub struct HayPedidoEnProcesoEnEseSlot {
    pub numero_slot: u8,
}

impl HayPedidoEnProcesoEnEseSlot {
    /// Prepara el identificador de error en bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        vec![TIPO_SLOT_EN_PROCESO, self.numero_slot]
    }

    /// Deserializa el error.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.len() < 2 || bytes[0] != TIPO_SLOT_EN_PROCESO {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        Ok(Self {
            numero_slot: bytes[1],
        })
    }
}
/// Respuesta TCP que confirma exitosamente la devolución de la unidad dentro del slot indicado.
pub struct BicicletaDevueltaCorrectamente;

impl BicicletaDevueltaCorrectamente {
    /// Prepara el identificador de éxito en bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        vec![TIPO_BICI_DEVUELTA_OK]
    }

    /// Deserializa el evento.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.is_empty() || bytes[0] != TIPO_BICI_DEVUELTA_OK {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        Ok(Self)
    }
}

/// Respuesta TCP negativa enviada si el usuario intentó devolver la bicicleta en un slot que ya posee una unidad físicamente.
pub struct NoSePudoDevolverBicicletaEnSlot {
    pub numero_slot: u8,
}

impl NoSePudoDevolverBicicletaEnSlot {
    /// Prepara el identificador de error en bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        vec![TIPO_BICI_DEVUELTA_ERR, self.numero_slot]
    }

    /// Deserializa el error.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.len() < 2 || bytes[0] != TIPO_BICI_DEVUELTA_ERR {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        Ok(Self {
            numero_slot: bytes[1],
        })
    }
}

/// Respuesta TCP que denota el rechazo transaccional de un depósito de seguridad al retirar una unidad.
pub struct PagoRechazado;

impl PagoRechazado {
    /// Prepara el código de rechazo en bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        vec![TIPO_PAGO_RECHAZADO]
    }

    /// Deserializa el evento de rechazo de pagos.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.is_empty() || bytes[0] != TIPO_PAGO_RECHAZADO {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        Ok(Self)
    }
}

/// Estructura devuelta por la Estación (UDP) al usuario con el desglose exacto de los lugares libres y ocupados en ella.
pub struct EnviarEstado {
    pub slots_libres: Vec<usize>, // índices de los slots libres, entre 0 y 19.
    pub slots_ocupados: Vec<usize>, // índices de los slots ocupados, entre 0 y 19.
}

impl EnviarEstado {
    /// Serializa los arreglos de libres y ocupados.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![TIPO_ENVIAR_ESTADO];
        bytes.extend(&(self.slots_libres.len() as u32).to_be_bytes());
        for slot in &self.slots_libres {
            bytes.extend(&slot.to_be_bytes());
        }
        bytes.extend(&(self.slots_ocupados.len() as u32).to_be_bytes());
        for slot in &self.slots_ocupados {
            bytes.extend(&slot.to_be_bytes());
        }
        bytes
    }

    /// Deserializa las listas de índices dinámicos recibidos por red de la estación.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.is_empty() || bytes[0] != TIPO_ENVIAR_ESTADO {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        let mut offset = 1;
        if bytes.len() < offset + 4 {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        let num_libres = u32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| ErrorParseo::NoSePudoParsear)?,
        ) as usize;
        offset += 4;
        let mut slots_libres = Vec::new();
        for _ in 0..num_libres {
            if bytes.len() < offset + 8 {
                return Err(ErrorParseo::NoSePudoParsear);
            }
            let slot = usize::from_be_bytes(
                bytes[offset..offset + 8]
                    .try_into()
                    .map_err(|_| ErrorParseo::NoSePudoParsear)?,
            );
            slots_libres.push(slot);
            offset += 8;
        }
        if bytes.len() < offset + 4 {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        let num_ocupados = u32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| ErrorParseo::NoSePudoParsear)?,
        ) as usize;
        offset += 4;
        let mut slots_ocupados = Vec::new();
        for _ in 0..num_ocupados {
            if bytes.len() < offset + 8 {
                return Err(ErrorParseo::NoSePudoParsear);
            }
            let slot = usize::from_be_bytes(
                bytes[offset..offset + 8]
                    .try_into()
                    .map_err(|_| ErrorParseo::NoSePudoParsear)?,
            );
            slots_ocupados.push(slot);
            offset += 8;
        }
        Ok(Self {
            slots_libres,
            slots_ocupados,
        })
    }
}

// Obtención y envío de líder

/// Mensaje UDP para que un usuario conozca cuál de las estaciones que conforman el anillo tiene rol de líder.
pub struct ObtenerLiderActual;

impl ObtenerLiderActual {
    /// Retorna el tipo de mensaje serializado.
    pub fn as_bytes(&self) -> Vec<u8> {
        vec![TIPO_SOLICITAR_LIDER]
    }

    /// Valida que los bytes recibidos calcen con la solicitud del líder.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.len() != 1 || bytes[0] != TIPO_SOLICITAR_LIDER {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        Ok(Self)
    }
}

/// Respuesta UDP donde una estación informa formalmente la identidad del líder actual a la aplicación.
pub struct EnviarLiderActual(pub usize);

impl EnviarLiderActual {
    /// Serializa el identificador numérico hacia bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![TIPO_ENVIAR_LIDER_ACTUAL];
        bytes.extend(&self.0.to_be_bytes());
        bytes
    }

    /// Parsea la respuesta que contiene el ID de la estación líder desde el arreglo de bytes de la red.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.is_empty() || bytes[0] != TIPO_ENVIAR_LIDER_ACTUAL {
            return Err(ErrorParseo::NoSePudoParsear);
        }

        Ok(EnviarLiderActual(usize::from_be_bytes(
            bytes[1..9]
                .try_into()
                .map_err(|_| ErrorParseo::NoSePudoParsear)?,
        )))
    }
}
