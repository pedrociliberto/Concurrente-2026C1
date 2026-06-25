use std::string::FromUtf8Error;

/// Enumeración que engloba los errores originados al intentar parsear (deserializar)
/// cadenas de bytes provenientes de la red hacia estructuras de datos del sistema.
#[derive(Debug)]
pub enum ErrorParseo {
    /// Se produce cuando los bytes provistos no coinciden con el formato,
    /// tamaño, índice o prefijo esperado para el mensaje dado.
    NoSePudoParsear,
}

impl From<FromUtf8Error> for ErrorParseo {
    fn from(_: FromUtf8Error) -> Self {
        ErrorParseo::NoSePudoParsear
    }
}
