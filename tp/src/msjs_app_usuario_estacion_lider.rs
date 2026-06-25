const TIPO_VISUALIZAR_ESTADO_ESTACIONES: u8 = 1;
const TIPO_ESTACIONES_PEDIDAS: u8 = 2;

// Envía AppUsuario:
// UDP
/// Mensaje UDP que envía la aplicación del usuario al sistema central (líder)
/// para solicitar información actualizada del estado de una o varias estaciones.
pub struct VisualizarEstadoEstaciones {
    pub estaciones: Vec<usize>, // IDs de las estaciones
}

impl VisualizarEstadoEstaciones {
    /// Serializa el mensaje a un vector de bytes para ser enviado por red.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(TIPO_VISUALIZAR_ESTADO_ESTACIONES);
        bytes.extend(&(self.estaciones.len() as u32).to_be_bytes());
        for id in &self.estaciones {
            bytes.extend(&(*id as u32).to_be_bytes());
        }
        bytes
    }

    /// Deserializa el mensaje desde un slice de bytes recibido por la red.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::error::ErrorParseo> {
        if bytes.is_empty() || bytes[0] != TIPO_VISUALIZAR_ESTADO_ESTACIONES {
            return Err(crate::error::ErrorParseo::NoSePudoParsear);
        }
        let num_estaciones = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;

        if bytes.len() < 5 + num_estaciones * 4 {
            return Err(crate::error::ErrorParseo::NoSePudoParsear);
        }

        let mut estaciones = Vec::new();
        for i in 0..num_estaciones {
            let base = 5 + i * 4;
            estaciones.push(u32::from_be_bytes([
                bytes[base],
                bytes[base + 1],
                bytes[base + 2],
                bytes[base + 3],
            ]) as usize);
        }
        Ok(VisualizarEstadoEstaciones { estaciones })
    }
}

// Envía SistemaCentral:

/// Mensaje UDP emitido por la estación líder en respuesta a una solicitud de estado.
/// Contiene un vector consolidado con la información y disponibilidad de las estaciones solicitadas.
#[derive(Clone, Debug, PartialEq)]
pub struct EstacionesPedidas {
    pub estaciones: Vec<EstacionInfo>,
}

impl EstacionesPedidas {
    /// Serializa la colección de información de estaciones a un vector de bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(TIPO_ESTACIONES_PEDIDAS);
        bytes.extend(&(self.estaciones.len() as u32).to_be_bytes());
        for estacion in &self.estaciones {
            bytes.extend(&(estacion.id as u32).to_be_bytes());
            bytes.extend(&(estacion.slots_libres as u32).to_be_bytes());
            bytes.extend(&(estacion.slots_ocupados as u32).to_be_bytes());
            let estado = match estacion.estado {
                EstacionEstado::Conectada => 1,
                EstacionEstado::Incierto => 2,
            };
            bytes.push(estado);
        }
        bytes
    }

    /// Deserializa la colección de estaciones desde un slice de bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::error::ErrorParseo> {
        if bytes.is_empty() || bytes[0] != TIPO_ESTACIONES_PEDIDAS {
            return Err(crate::error::ErrorParseo::NoSePudoParsear);
        }
        let num_estaciones = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;

        if bytes.len() < 5 + num_estaciones * 13 {
            return Err(crate::error::ErrorParseo::NoSePudoParsear);
        }

        let mut estaciones = Vec::new();
        for i in 0..num_estaciones {
            let base = 5 + i * 13;

            let id = u32::from_be_bytes([
                bytes[base],
                bytes[base + 1],
                bytes[base + 2],
                bytes[base + 3],
            ]) as usize;
            let slots_libres = u32::from_be_bytes([
                bytes[base + 4],
                bytes[base + 5],
                bytes[base + 6],
                bytes[base + 7],
            ]) as usize;
            let slots_ocupados = u32::from_be_bytes([
                bytes[base + 8],
                bytes[base + 9],
                bytes[base + 10],
                bytes[base + 11],
            ]) as usize;
            let estado = match bytes[base + 12] {
                1 => EstacionEstado::Conectada,
                2 => EstacionEstado::Incierto,
                _ => return Err(crate::error::ErrorParseo::NoSePudoParsear),
            };
            estaciones.push(EstacionInfo {
                id,
                slots_libres,
                slots_ocupados,
                estado,
            });
        }
        Ok(EstacionesPedidas { estaciones })
    }
}

/// Representa el estado actual de conectividad operativa de una estación en el sistema central.
#[derive(Clone, Debug, PartialEq)]
pub enum EstacionEstado {
    /// La estación responde y está conectada con el líder.
    Conectada,
    /// La estación no está conectada o se encuentra caída.
    Incierto,
}

/// Contiene el estado de los slots y conectividad asociados a una estación.
#[derive(Clone, Debug, PartialEq)]
pub struct EstacionInfo {
    pub id: usize,
    pub slots_libres: usize,
    pub slots_ocupados: usize,
    pub estado: EstacionEstado,
}
