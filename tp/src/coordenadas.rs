use std::fmt::Display;

/// Representa un par de coordenadas en un plano 2D, utilizadas para establecer
/// ubicaciones geográficas de usuarios y estaciones.
///
/// # Atributos
/// - `latitud`: Valor entero que representa la ubicación en el eje de las ordenadas (Y).
/// - `longitud`: Valor entero que representa la ubicación en el eje de las abscisas (X).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Coordenadas {
    latitud: isize,
    longitud: isize,
}

impl Coordenadas {
    /// Crea una nueva instancia de coordenadas.
    pub fn new(latitud: isize, longitud: isize) -> Self {
        Self { latitud, longitud }
    }

    /// Devuelve el valor de la latitud.
    pub fn latitud(&self) -> isize {
        self.latitud
    }

    /// Devuelve el valor de la longitud.
    pub fn longitud(&self) -> isize {
        self.longitud
    }

    /// Calcula la distancia euclidiana directa desde estas coordenadas hacia otras proporcionadas.
    ///
    /// # Parámetros
    /// - `otra_coord`: Objeto `Coordenadas` de destino.
    ///
    /// # Retornos
    /// - `f64`: La distancia escalar calculada.
    pub fn distancia(&self, otra_coord: Coordenadas) -> f64 {
        let dx = (self.latitud - otra_coord.latitud) as f64;
        let dy = (self.longitud - otra_coord.longitud) as f64;
        (dx * dx + dy * dy).sqrt()
    }
}

impl Display for Coordenadas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.longitud, self.latitud)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test01_creacion_y_accesos() {
        let coord = Coordenadas::new(10, -5);
        assert_eq!(coord.latitud(), 10);
        assert_eq!(coord.longitud(), -5);
    }

    #[test]
    fn test02_distancia_euclidiana() {
        let c1 = Coordenadas::new(0, 0);
        let c2 = Coordenadas::new(3, 4);

        // La distancia pitagórica entre (0,0) y (3,4) es la hipotenusa de un triángulo 3-4-5.
        assert_eq!(c1.distancia(c2), 5.0);
    }

    #[test]
    fn test03_formateo_display() {
        let coord = Coordenadas::new(12, 34);
        assert_eq!(format!("{}", coord), "(34, 12)");
    }
}
