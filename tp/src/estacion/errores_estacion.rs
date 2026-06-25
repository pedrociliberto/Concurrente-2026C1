//! errores_estacion.rs
//!
//! Módulo que define los errores específicos que pueden ocurrir durante la ejecución del proceso de una estación.
//!

/// Enumeración que engloba todos los posibles errores que pueden ocurrir en el subsistema de la estación.
#[derive(Debug)]
pub enum EstacionError {
    /// Ocurre cuando no se encuentra el archivo de configuración principal de estaciones en la ruta especificada.
    ConfigFileNotFound(String),
    /// Ocurre cuando una línea del archivo de configuración no respeta el formato esperado.
    ConfigParseError(String),
    /// Ocurre cuando el archivo existe, pero no contiene los datos (la fila) para el ID proporcionado al arrancar.
    StationConfigNotFound(usize),
    /// Ocurre cuando una dirección IP o puerto tiene un formato inválido o no puede ser parseado.
    InvalidAddress(String),
    /// Envuelve errores estándar de Entrada/Salida (`std::io::Error`), aplicable a lectura de archivos o flujos de red.
    IoError(std::io::Error),
    /// Ocurre ante un problema general a nivel de red, como el fallo al enlazar un socket (bind).
    NetworkError(String),
    /// Ocurre cuando los argumentos suministrados por la línea de comandos son incorrectos o insuficientes.
    InvalidArgs,
    /// Envuelve los errores generados por Actix (`actix::MailboxError`) al intentar enviar un mensaje a un actor caído o saturado.
    MailboxError(actix::MailboxError),
    /// Ocurre cuando se intenta operar con un canal asíncrono de Rust (e.g. `mpsc`) que ya ha sido cerrado.
    CanalCerrado,
}

impl std::fmt::Display for EstacionError {
    /// Formatea el error para generar un mensaje descriptivo que sea fácil de entender en los logs del sistema.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigFileNotFound(msg) => {
                write!(f, "Archivo de configuración no encontrado: {msg}")
            }
            Self::ConfigParseError(msg) => write!(f, "Error parseando configuración: {msg}"),
            Self::StationConfigNotFound(id) => {
                write!(f, "No se encontró la configuración para la estación {id}")
            }
            Self::InvalidAddress(addr) => write!(f, "Dirección IP o puerto inválido: {addr}"),
            Self::IoError(e) => write!(f, "Error de Entrada/Salida: {e}"),
            Self::NetworkError(msg) => write!(f, "Error de red/comunicación: {msg}"),
            Self::InvalidArgs => write!(f, "Argumentos de línea de comandos inválidos o faltantes"),
            Self::MailboxError(e) => write!(f, "Error al enviar mensaje: {e}"),
            Self::CanalCerrado => write!(f, "El canal ha sido cerrado"),
        }
    }
}

impl std::error::Error for EstacionError {}

impl From<actix::MailboxError> for EstacionError {
    /// Convierte de forma automática un error de buzón de Actix al tipo general `EstacionError`.
    fn from(err: actix::MailboxError) -> Self {
        EstacionError::MailboxError(err)
    }
}

impl From<std::io::Error> for EstacionError {
    /// Convierte de forma automática un error de Entrada/Salida estándar al tipo general `EstacionError`.
    fn from(err: std::io::Error) -> Self {
        EstacionError::IoError(err)
    }
}

impl From<std::net::AddrParseError> for EstacionError {
    /// Convierte de forma automática un error de parseo de direcciones IP al tipo general `EstacionError`.
    fn from(err: std::net::AddrParseError) -> Self {
        EstacionError::InvalidAddress(err.to_string())
    }
}
