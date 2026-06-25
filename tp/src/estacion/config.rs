//! config.rs
//!
//! Este módulo se encarga de crear el actor estación a partir de un id proporcionado.
//! Se encarga tanto de leer la configuración inicial de la estación desde un archivo,
//! como de recuperar el estado previo de la estación (si existe) desde otro archivo.
//!

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs::read_to_string,
    time::Instant,
};

use tp::{
    constantes::{ADDR_BASE, CANTIDAD_ESTACIONES, PUERTO_BASE_PROCESADOR_PAGOS},
    coordenadas::Coordenadas,
    msjs_app_usuario_estacion::{Bicicleta, EstadoBicicleta},
};

use crate::actor::{Estacion, EstadoSlot};
use crate::errores_estacion::EstacionError;

const ARCHIVO_CONFIG: &str = "src/estaciones.config";
const ARCHIVO_CONFIG_CANT_COLUMNAS: usize = 5;

const DIR_ESTADOS: &str = "src/estado_estaciones";
const DIR_ESTADOS_TEST: &str = "src/test_estado_estaciones";
const INDICE_ID: usize = 0;
const INDICE_NOMBRE: usize = 1;
const INDICE_LATITUD: usize = 2;
const INDICE_LONGITUD: usize = 3;
const INDICE_SLOTS: usize = 4;
const INDICE_ID_BICI: usize = 1;
const INDICE_ESTADO_BICI: usize = 2;
const INDICE_USUARIO_BICI: usize = 3;

const VACIO: &str = "VACIO";
const OCUPADO: &str = "OCUPADO";
const EN_USO: &str = "EnUso";

struct ConfiguracionEstacion {
    id: usize,
    nombre: String,
    latitud: isize,
    longitud: isize,
    cant_slots: usize,
}

/// Crea un actor estación a partir de su id.
///
/// La configuración de la estación se lee del archivo de configuración.
/// Además, si esta estación tiene un estado previo guardado, se recupera
/// del archivo correspondiente.
///
/// # Panics
///
/// - Si no se puede abrir el archivo de configuración.
/// - Si no se encuentra la configuración para la estación dada.
pub fn crear_actor_estacion(id: usize, conectado: bool) -> Result<Estacion, EstacionError> {
    let contenido_config = std::fs::read_to_string(ARCHIVO_CONFIG)
        .map_err(|e| EstacionError::ConfigFileNotFound(format!("{} -> {}", ARCHIVO_CONFIG, e)))?;

    let mut lineas = contenido_config.lines().skip(1);
    let config = lineas
        .find_map(|linea| parsear_configuracion(id, linea))
        .ok_or(EstacionError::StationConfigNotFound(id))?;

    let slots_iniciales = recuperar_estado(id, config.cant_slots);
    let procesador_addr_str = format!("{ADDR_BASE}:{PUERTO_BASE_PROCESADOR_PAGOS}");
    let procesador_de_pagos = procesador_addr_str
        .parse()
        .map_err(|_| EstacionError::InvalidAddress(procesador_addr_str))?;

    Ok(Estacion {
        id: config.id,
        nombre: config.nombre,
        slots: slots_iniciales,
        coordenadas: Coordenadas::new(config.latitud, config.longitud),
        tx_tcp: None,
        otras_estaciones: generar_hashset_con_otras_estaciones(id),
        conectado,
        lider_actual: None,
        procesador_de_pagos,
        estaciones_info: Vec::new(),
        ring_eleccion: None,
        servidor_tcp_iniciado: false,
        seguidores_tx: HashMap::new(),
        alquileres_activos: HashMap::new(),
        pagos_pendientes: VecDeque::new(),
    })
}

/// Parsea una línea del archivo de configuración y devuelve una ConfiguracionEstacion
/// de la estación cuyo id coincide con el proporcionado.
///
/// Retorna None si la línea no tiene el formato esperado o si el id no coincide.
fn parsear_configuracion(id_estacion: usize, linea: &str) -> Option<ConfiguracionEstacion> {
    let datos: Vec<&str> = linea.split(',').map(|s| s.trim()).collect();

    if datos.len() == ARCHIVO_CONFIG_CANT_COLUMNAS
        && let (Ok(id), Ok(latitud), Ok(longitud), Ok(cant_slots)) = (
            datos[INDICE_ID].parse::<usize>(),
            datos[INDICE_LATITUD].parse::<isize>(),
            datos[INDICE_LONGITUD].parse::<isize>(),
            datos[INDICE_SLOTS].parse::<usize>(),
        )
        && id == id_estacion
    {
        return Some(ConfiguracionEstacion {
            id,
            nombre: datos[INDICE_NOMBRE].to_string(),
            latitud,
            longitud,
            cant_slots,
        });
    }
    None
}

/// Recupera el estado de los slots de una estación a partir de un archivo de estado
/// específico para esa estación.
fn recuperar_estado(id_estacion: usize, cant_slots: usize) -> Vec<EstadoSlot> {
    let dir_estados = if cfg!(test) {
        DIR_ESTADOS_TEST
    } else {
        DIR_ESTADOS
    };

    let nombre_archivo_estado = format!("{}/estacion_{}.state", dir_estados, id_estacion);

    if let Ok(contenido) = read_to_string(&nombre_archivo_estado) {
        println!(
            "[Estación {}] Archivo de estado detectado. Recuperando slots...",
            id_estacion
        );

        let mut recuperados = Vec::new();

        for line in contenido.lines() {
            if line == VACIO {
                recuperados.push(EstadoSlot::Vacio);
            } else if line.starts_with(OCUPADO) {
                let datos_bici: Vec<&str> = line.split(',').collect();
                if let Ok(id_bici) = datos_bici[INDICE_ID_BICI].parse::<usize>() {
                    let estado_bici = if datos_bici[INDICE_ESTADO_BICI] == EN_USO {
                        let id_usuario = datos_bici[INDICE_USUARIO_BICI]
                            .parse::<usize>()
                            .unwrap_or(0);
                        EstadoBicicleta::EnUso(Instant::now(), id_usuario) // TODO: se podría guardar el instante de inicio del uso en el archiva??
                    } else {
                        EstadoBicicleta::Disponible
                    };

                    recuperados.push(EstadoSlot::Ocupado(Bicicleta::new(id_bici, estado_bici)));
                }
            }
        }
        recuperados
    } else {
        println!(
            "[Estación {}] No se encontró estado previo. Inicializando por defecto.",
            id_estacion
        );
        agregar_bicicletas(id_estacion, cant_slots)
    }
}

/// Genera un HashSet con los ids de las otras estaciones, excluyendo el id de la estación actual.
fn generar_hashset_con_otras_estaciones(id_estacion: usize) -> HashSet<usize> {
    let mut hashset = HashSet::new();
    for i in 1..=CANTIDAD_ESTACIONES {
        if i != id_estacion {
            hashset.insert(i);
        }
    }
    hashset
}

/// Agrega bicicletas a los slots de la estación, asignándoles ids únicos basados en el id de la estación
/// y su posición.
fn agregar_bicicletas(id_estacion: usize, cantidad: usize) -> Vec<EstadoSlot> {
    let mut slots = Vec::new();
    for i in 0..cantidad {
        let bicicleta = Bicicleta::new(id_estacion * 100 + i + 1, EstadoBicicleta::Disponible);
        slots.push(EstadoSlot::Ocupado(bicicleta));
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::EstadoSlot;
    use std::fs::{File, create_dir_all, remove_file};
    use std::io::Write;
    use std::path::Path;
    use tp::msjs_app_usuario_estacion::EstadoBicicleta;

    fn limpiar_archivo_test(id_estacion: usize) {
        let path = format!("{}/estacion_{}.state", DIR_ESTADOS_TEST, id_estacion);
        if Path::new(&path).exists() {
            let _ = remove_file(path);
        }
    }

    #[test]
    fn test01_generar_hashset_excluye_id_propio_y_tiene_tamano_correcto() {
        let id_propio = 3;
        let estaciones_vecinas = generar_hashset_con_otras_estaciones(id_propio);

        assert!(!estaciones_vecinas.contains(&id_propio));
        assert_eq!(
            estaciones_vecinas.len(),
            tp::constantes::CANTIDAD_ESTACIONES - 1
        );
    }

    #[test]
    fn test02_recuperar_estado_sin_archivo_previo_inicializa_por_defecto() {
        let id_estacion_test = 999;
        limpiar_archivo_test(id_estacion_test);

        let cant_slots = 3;
        let slots = recuperar_estado(id_estacion_test, cant_slots);

        assert_eq!(slots.len(), cant_slots);

        for slot in slots {
            match slot {
                EstadoSlot::Ocupado(bici) => {
                    assert!(matches!(bici.estado, EstadoBicicleta::Disponible));
                }
                _ => panic!(
                    "Se esperaba que los slots iniciales por defecto estuvieran Ocupados con bicis disponibles"
                ),
            }
        }
    }

    #[test]
    fn test03_recuperar_estado_con_archivo_existente_reconstruye_slots_correctamente() {
        let id_estacion_test = 888;
        limpiar_archivo_test(id_estacion_test);

        let carpeta = DIR_ESTADOS_TEST;
        create_dir_all(carpeta).unwrap();
        let path = format!("{}/estacion_{}.state", carpeta, id_estacion_test);

        let mut archivo = File::create(&path).unwrap();
        writeln!(archivo, "VACIO").unwrap();
        writeln!(archivo, "OCUPADO,105,Disponible").unwrap();
        writeln!(archivo, "OCUPADO,202,EnUso,42,Instant").unwrap(); // Usuario 42 usando la bici 202
        archivo.flush().unwrap();

        let slots = recuperar_estado(id_estacion_test, 3);

        assert_eq!(slots.len(), 3);

        assert!(matches!(slots[0], EstadoSlot::Vacio));

        if let EstadoSlot::Ocupado(ref bici) = slots[1] {
            assert_eq!(bici.id, 105);
            assert!(matches!(bici.estado, EstadoBicicleta::Disponible));
        } else {
            panic!("Slot 2 incorrecto");
        }

        if let EstadoSlot::Ocupado(ref bici) = slots[2] {
            assert_eq!(bici.id, 202);
            if let EstadoBicicleta::EnUso(_, id_usuario) = bici.estado {
                assert_eq!(id_usuario, 42);
            } else {
                panic!("La bicicleta debió listarse EnUso");
            }
        } else {
            panic!("Slot 3 incorrecto");
        }

        limpiar_archivo_test(id_estacion_test);
    }
}
