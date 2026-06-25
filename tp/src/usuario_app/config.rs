//! config.rs
//!
//! Este módulo se encarga de crear el actor de la aplicación del usuario, cargando la información de las estaciones
//! desde un archivo de configuración y recuperando el estado del usuario desde un archivo específico si existe.
//!

use crate::actor::UsuarioApp;
use rand::RngExt;
use std::{collections::HashMap, fs::read_to_string, net::SocketAddr};
use tp::{
    constantes::{ADDR_BASE, PUERTO_BASE_ESTACION},
    coordenadas::Coordenadas,
    msjs_app_usuario_estacion::Bicicleta,
    objetos_bancarios::TarjetaDeCredito,
};

pub const DIR_ESTADO_USUARIOS: &str = "src/estado_usuarios";
const INDICE_LONGITUD: usize = 0;
const INDICE_LATITUD: usize = 1;
const LINEA_COORDENADAS: usize = 0;
const LINEA_TARJETA: usize = 1;
const LINEA_INICIO_BICICLETAS: usize = 2;

/// Carga la información de las estaciones desde el archivo `estaciones.config`, creando un `HashMap` donde
/// la clave es el ID de la estación
///
/// Si el archivo no se puede abrir, se muestra un mensaje de error y el programa termina.
///
/// Si alguna línea del archivo está mal formateada, se muestra un mensaje de error indicando la línea problemática,
/// pero el programa continúa procesando las demás líneas válidas.
fn cargar_estaciones() -> Result<HashMap<usize, (String, Coordenadas, SocketAddr)>, std::io::Error>
{
    Ok(std::fs::read_to_string("src/estaciones.config")?
        .lines()
        .skip(1)
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() != 5 {
                eprintln!("Línea mal formateada en estaciones.txt: {}", line);
                return None;
            }
            let id = parts[0].parse::<usize>().ok()?;
            let nombre = parts[1].to_string();
            let latitud = parts[2].parse::<isize>().ok()?;
            let longitud = parts[3].parse::<isize>().ok()?;
            let puerto = PUERTO_BASE_ESTACION + id as u16; // Asigna un puerto basado en el ID de la estación
            Some((
                id,
                (
                    nombre,
                    Coordenadas::new(latitud, longitud),
                    match format!("{}:{}", ADDR_BASE, puerto).parse::<SocketAddr>() {
                        Ok(addr) => addr,
                        Err(e) => {
                            eprintln!(
                                "\x1b[31mError al parsear la dirección de una estación: {}\x1b[0m",
                                e
                            );
                            return None;
                        }
                    },
                ),
            ))
        })
        .collect())
}

/// Crea el actor de la aplicación del usuario, con su tarjeta de crédito y la dirección del
/// sistema central.
///
/// Si existe un archivo de estado para el usuario, se recupera su información (coordenadas, tarjeta de crédito
/// y bicicletas en uso) desde dicho archivo.
///
/// Si el archivo de estado no existe o está mal formateado, se inicializa el usuario con las coordenadas proporcionadas,
/// una tarjeta de crédito generada aleatoriamente y sin bicicletas en uso.
pub fn crear_actor_usuario_app(
    id: usize,
    longitud: Option<isize>,
    latitud: Option<isize>,
) -> UsuarioApp {
    let estaciones = cargar_estaciones().unwrap_or_default();

    let ruta_archivo_estado = format!("{}/estado_usuario_{}.state", DIR_ESTADO_USUARIOS, id);

    let mut coordenadas = None;
    if let Some(longitud) = longitud
        && let Some(latitud) = latitud
    {
        coordenadas = Some(Coordenadas::new(latitud, longitud));
    }

    let mut rng = rand::rng();
    let numero_tarjeta = (0..16)
        .map(|_| rng.random_range(0..10).to_string())
        .collect::<String>();
    let cod_seguridad = rng.random_range(100..1000);
    let vencimiento = format!(
        "{:02}/{:02}",
        rng.random_range(1..13),
        rng.random_range(24..30)
    );
    let mut tarjeta = TarjetaDeCredito::new(&numero_tarjeta, cod_seguridad, &vencimiento);

    let mut bicicletas_en_uso = HashMap::new();

    if let Ok(contenido) = read_to_string(&ruta_archivo_estado) {
        println!(
            "Recuperando estado del usuario desde el archivo: {}",
            ruta_archivo_estado
        );

        let lineas: Vec<&str> = contenido.lines().collect();

        let linea_coordenadas = lineas.get(LINEA_COORDENADAS).unwrap_or(&"");
        let datos: Vec<&str> = linea_coordenadas.split(',').map(|s| s.trim()).collect();
        let longitud_recuperada = datos
            .get(INDICE_LONGITUD)
            .and_then(|s| s.parse::<isize>().ok())
            .unwrap_or(0);
        let latitud_recuperada = datos
            .get(INDICE_LATITUD)
            .and_then(|s| s.parse::<isize>().ok())
            .unwrap_or(0);
        if coordenadas.is_none() {
            coordenadas = Some(Coordenadas::new(latitud_recuperada, longitud_recuperada));
        }

        let linea_tarjeta = lineas.get(LINEA_TARJETA).unwrap_or(&"");
        let bytes_tarjeta = linea_tarjeta
            .split(',')
            .map(|s| s.trim().parse::<u8>().ok())
            .collect::<Option<Vec<u8>>>();
        tarjeta =
            TarjetaDeCredito::from_bytes(&bytes_tarjeta.unwrap_or_default()).unwrap_or(tarjeta);

        bicicletas_en_uso = contenido
            .lines()
            .skip(LINEA_INICIO_BICICLETAS)
            .filter_map(|linea| {
                let bytes = match linea
                    .split(',')
                    .map(|s| s.parse::<u8>().ok())
                    .collect::<Option<Vec<u8>>>()
                {
                    Some(bytes) => bytes,
                    None => {
                        eprintln!(
                            "Línea mal formateada en el archivo de estado del usuario: {}",
                            linea
                        );
                        return None;
                    }
                };
                if let Ok(bicicleta) = Bicicleta::from_bytes(&bytes) {
                    Some((bicicleta.id, bicicleta))
                } else {
                    eprintln!(
                        "Línea mal formateada en el archivo de estado del usuario: {}",
                        linea
                    );
                    None
                }
            })
            .collect();
    }

    UsuarioApp::new(
        id,
        coordenadas.unwrap_or(Coordenadas::new(0, 0)),
        None,
        tarjeta,
        estaciones,
        bicicletas_en_uso,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::remove_file, time::Instant};
    use tp::{constantes::CANTIDAD_ESTACIONES, msjs_app_usuario_estacion::EstadoBicicleta};
    const ID_BASE: usize = 1000; // ID base para evitar conflictos con usuarios reales

    #[test]
    fn test01_crear_actor_usuario_app_sin_archivo_estado() {
        let id = ID_BASE + 1;
        let usuario_app = crear_actor_usuario_app(id, Some(10), Some(10));
        assert_eq!(usuario_app.id, id);
        assert_eq!(usuario_app.coordenadas.latitud(), 10);
        assert_eq!(usuario_app.coordenadas.longitud(), 10);
        assert!(usuario_app.bicicletas_en_uso.is_empty());
    }

    #[test]
    fn test02_crear_actor_usuario_app_con_archivo_estado_sin_bicicletas() {
        let id = ID_BASE + 2;
        let longitud = 20;
        let latitud = 20;
        let numero_tarjeta = "1234567890123456";
        let cod_seguridad = 123;
        let vencimiento = "12/25";
        let usuario_app_ant = UsuarioApp::new(
            id,
            Coordenadas::new(longitud, latitud),
            None,
            TarjetaDeCredito::new(numero_tarjeta, cod_seguridad, vencimiento),
            HashMap::new(),
            HashMap::new(),
        );
        usuario_app_ant.guardar_estado();

        let usuario_app = crear_actor_usuario_app(id, Some(longitud), Some(latitud));
        assert_eq!(usuario_app.id, id);
        assert_eq!(usuario_app.coordenadas.latitud(), latitud);
        assert_eq!(usuario_app.coordenadas.longitud(), longitud);
        assert_eq!(usuario_app.tarjeta_de_credito.numero, numero_tarjeta);
        assert_eq!(usuario_app.tarjeta_de_credito.cod_seguridad, cod_seguridad);
        assert_eq!(usuario_app.tarjeta_de_credito.vencimiento, vencimiento);
        assert!(usuario_app.bicicletas_en_uso.is_empty());

        // Elimino archivo
        remove_file(format!(
            "{}/estado_usuario_{}.state",
            DIR_ESTADO_USUARIOS, id
        ))
        .unwrap();
    }

    #[test]
    fn test03_crear_actor_usuario_app_con_archivo_estado_con_bicicletas() {
        let id = ID_BASE + 3;
        let mut bicicletas_en_uso = HashMap::new();
        for i in 0..5 {
            bicicletas_en_uso.insert(
                i,
                Bicicleta::new(i, EstadoBicicleta::EnUso(Instant::now(), id)),
            );
        }
        let usuario_app_ant = UsuarioApp::new(
            id,
            Coordenadas::new(30, 30),
            None,
            TarjetaDeCredito::new("1234567890123456", 123, "12/25"),
            HashMap::new(),
            bicicletas_en_uso.clone(),
        );
        usuario_app_ant.guardar_estado();

        let usuario_app = crear_actor_usuario_app(id, Some(30), Some(30));
        assert_eq!(usuario_app.id, id);
        assert_eq!(usuario_app.coordenadas.latitud(), 30);
        assert_eq!(usuario_app.coordenadas.longitud(), 30);
        assert_eq!(usuario_app.tarjeta_de_credito.numero, "1234567890123456");
        assert_eq!(usuario_app.tarjeta_de_credito.cod_seguridad, 123);
        assert_eq!(usuario_app.tarjeta_de_credito.vencimiento, "12/25");
        assert_eq!(usuario_app.bicicletas_en_uso.len(), 5);
        bicicletas_en_uso
            .iter()
            .for_each(|(id_bicicleta_esperado, _)| {
                assert!(
                    usuario_app
                        .bicicletas_en_uso
                        .contains_key(id_bicicleta_esperado)
                );
                let bicicleta = usuario_app
                    .bicicletas_en_uso
                    .get(id_bicicleta_esperado)
                    .unwrap();
                assert_eq!(bicicleta.id, *id_bicicleta_esperado);
                assert!(matches!(bicicleta.estado, EstadoBicicleta::EnUso(_, _)));
            });

        // Elimino archivo
        remove_file(format!(
            "{}/estado_usuario_{}.state",
            DIR_ESTADO_USUARIOS, id
        ))
        .unwrap();
    }

    #[test]
    fn test04_cargar_estaciones() {
        let estaciones = cargar_estaciones().unwrap();
        assert_eq!(estaciones.len(), CANTIDAD_ESTACIONES);
    }
}
