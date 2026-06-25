use crate::error::ErrorParseo;

/// Representa una tarjeta de crédito utilizada por los usuarios del sistema.
///
/// # Atributos
/// - `numero`: Identificador frontal numérico de 16 caracteres.
/// - `cod_seguridad`: Código CVV numérico que autentica la tarjeta.
/// - `vencimiento`: Fecha de expiración de la credencial en formato cadena.
#[derive(Clone, Debug)]
pub struct TarjetaDeCredito {
    pub numero: String,
    pub cod_seguridad: u16,
    pub vencimiento: String,
}

const BYTES_TARJETA_CREDITO: usize = 23;

impl TarjetaDeCredito {
    /// Constructor base de un nuevo objeto de Tarjeta de Crédito.
    pub fn new(numero: &str, cod_seguridad: u16, vencimiento: &str) -> Self {
        TarjetaDeCredito {
            numero: numero.to_string(),
            cod_seguridad,
            vencimiento: vencimiento.to_string(),
        }
    }

    /// Serializa los datos en un vector compacto de bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.numero.as_bytes());
        bytes.extend_from_slice(&self.cod_seguridad.to_be_bytes());
        bytes.extend_from_slice(self.vencimiento.as_bytes());
        bytes
    }

    /// Instancia una `TarjetaDeCredito` basándose en los bytes recibidos.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorParseo> {
        if bytes.len() != BYTES_TARJETA_CREDITO {
            return Err(ErrorParseo::NoSePudoParsear);
        }
        let numero = String::from_utf8(bytes[0..16].to_vec())?;
        let cod_seguridad = u16::from_be_bytes([bytes[16], bytes[17]]);
        let vencimiento = String::from_utf8(bytes[18..].to_vec())?;
        Ok(TarjetaDeCredito {
            numero,
            cod_seguridad,
            vencimiento,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test01_creacion_y_serializacion() {
        let tarjeta = TarjetaDeCredito::new("1234567890123456", 123, "12/25");
        let bytes = tarjeta.as_bytes();

        assert_eq!(
            bytes.len(),
            BYTES_TARJETA_CREDITO,
            "La longitud en bytes debería ser exactamente de 23 bytes"
        );
        assert_eq!(
            &bytes[0..16],
            b"1234567890123456",
            "El número de tarjeta no coincide en el empaquetado"
        );
        assert_eq!(
            u16::from_be_bytes([bytes[16], bytes[17]]),
            123,
            "El código CVV no coincide en el empaquetado"
        );
        assert_eq!(
            &bytes[18..],
            b"12/25",
            "El vencimiento no coincide en el empaquetado"
        );
    }

    #[test]
    fn test02_deserializacion_exitosa() {
        let tarjeta_original = TarjetaDeCredito::new("9876543210987654", 999, "01/30");
        let bytes = tarjeta_original.as_bytes();

        let tarjeta_recuperada =
            TarjetaDeCredito::from_bytes(&bytes).expect("Debería parsear los bytes correctamente");

        assert_eq!(tarjeta_recuperada.numero, "9876543210987654");
        assert_eq!(tarjeta_recuperada.cod_seguridad, 999);
        assert_eq!(tarjeta_recuperada.vencimiento, "01/30");
    }

    #[test]
    fn test03_deserializacion_falla_por_longitud_incorrecta() {
        let bytes_cortos = vec![0u8; 20]; // Solo le damos 20 bytes en lugar de los 23 esperados
        let resultado = TarjetaDeCredito::from_bytes(&bytes_cortos);
        assert!(
            matches!(resultado.unwrap_err(), ErrorParseo::NoSePudoParsear),
            "Debería fallar al intentar parsear un slice con longitud incorrecta"
        );
    }
}
