//! comandos.rs
//!
//! Este módulo define los comandos que el usuario puede ingresar en la aplicación, junto con sus formatos y descripciones.
//! También implementa las funciones que ejecutan cada comando, interactuando con el actor `UsuarioApp` para realizar las
//! acciones correspondientes.
//!

use crate::actor::UsuarioApp;
use crate::mensajes_internos::{
    ActualizarCoordenadas, CambiarConectividad, FinalizarAlquilerBicicleta,
    IniciarAlquilerBicicleta, ListarBicicletasEnUso, SolicitarEstadoEstacion,
    VerEstacionesExistentes, VisualizarInfoUsuario,
};
use actix::Addr;
use std::time::{SystemTime, UNIX_EPOCH};
use tp::{
    coordenadas::Coordenadas,
    msjs_app_usuario_estacion::{EnviarEstado, EstadoBicicleta},
    msjs_app_usuario_estacion_lider::EstacionEstado,
};

use crate::mensajes_internos::SolicitarEstadoEstaciones;

/// Estructura que representa un comando que el usuario puede ingresar en la aplicación,
/// con su descripción, formato y número de comando.
pub struct Comando {
    pub descripcion: &'static str,
    pub formato: &'static str,
    pub numero: &'static str,
}

impl Comando {
    /// Calcula la cantidad mínima de argumentos requeridos para el comando, contando solo aquellos que no
    /// son opcionales ni repetitivos.
    fn cantidad_argumentos_min(&self) -> usize {
        self.formato
            .split_whitespace()
            .skip(1)
            .filter(|s| !s.starts_with('['))
            .filter(|s| !s.contains("..."))
            .filter(|s| !s.ends_with(']'))
            .count()
    }

    /// Calcula la cantidad máxima de argumentos permitidos para el comando, considerando que si el
    /// formato contiene "..." entonces no hay un límite superior, de lo contrario cuenta los argumentos
    /// definidos en el formato, incluyendo opcionales.
    fn cantidad_argumentos_max(&self) -> usize {
        if self.formato.contains("...") {
            usize::MAX
        } else {
            self.formato.split_whitespace().skip(1).count()
        }
    }

    /// Valida si la cantidad de argumentos proporcionados para el comando es adecuada según su formato,
    /// verificando que esté entre la cantidad mínima y máxima calculada.
    fn validar_argumentos(&self, argumentos: &[&str]) -> bool {
        argumentos.len() >= self.cantidad_argumentos_min()
            && argumentos.len() <= self.cantidad_argumentos_max()
    }
}

pub const IMPRIMIR_AYUDA: Comando = Comando {
    numero: "0",
    descripcion: "Mostrar comandos disponibles",
    formato: "0",
};

pub const VISUALIZAR_ESTACIONES: Comando = Comando {
    numero: "1",
    descripcion: "Visualizar coordenadas de todas las estaciones o a menos de una distancia dada (en km)",
    formato: "1 [<distanciaMax>]",
};

pub const LISTAR_BICICLETAS_EN_USO: Comando = Comando {
    numero: "2",
    descripcion: "Listar bicicletas en uso (alquileres activos)",
    formato: "2",
};

pub const VISUALIZAR_ESTADO_ESTACIONES: Comando = Comando {
    numero: "3",
    descripcion: "Visualizar estado de estaciones (IDs)",
    formato: "3 <idEst1> [<idEst2> ... <idEstN>]",
};

pub const SOLICITAR_ESTADO_ESTACION: Comando = Comando {
    numero: "4",
    descripcion: "Solicitar estado de slots de una estación (ID)",
    formato: "4 <idEst>",
};

pub const INICIAR_ALQUILER_BICICLETA: Comando = Comando {
    numero: "5",
    descripcion: "Iniciar alquiler de bicicleta (ID est. y núm. slot)",
    formato: "5 <idEst> <numSlot>",
};

pub const FINALIZAR_ALQUILER_BICICLETA: Comando = Comando {
    numero: "6",
    descripcion: "Finalizar alquiler de bicicleta (ID est., núm. slot e ID bici.)",
    formato: "6 <idEst> <numSlot> <idBici>",
};

pub const ACTUALIZAR_COORDENADAS: Comando = Comando {
    numero: "7",
    descripcion: "Actualizar las coordenadas actuales",
    formato: "7 <latitud> <longitud>",
};

pub const VISUALIZAR_INFO_USUARIO: Comando = Comando {
    numero: "8",
    descripcion: "Visualizar información del usuario",
    formato: "8",
};

pub const CAMBIAR_CONECTIVIDAD: Comando = Comando {
    numero: "9",
    descripcion: "Cambiar estado de conectividad",
    formato: "9",
};

pub const SALIR: Comando = Comando {
    numero: "10",
    descripcion: "Salir",
    formato: "10",
};

pub const COMANDOS: [Comando; 11] = [
    IMPRIMIR_AYUDA,
    VISUALIZAR_ESTACIONES,
    LISTAR_BICICLETAS_EN_USO,
    VISUALIZAR_ESTADO_ESTACIONES,
    SOLICITAR_ESTADO_ESTACION,
    INICIAR_ALQUILER_BICICLETA,
    FINALIZAR_ALQUILER_BICICLETA,
    ACTUALIZAR_COORDENADAS,
    VISUALIZAR_INFO_USUARIO,
    CAMBIAR_CONECTIVIDAD,
    SALIR,
];

/// Función que ejecuta el comando para visualizar las estaciones disponibles, mostrando su ID, nombre,
/// coordenadas y distancia al usuario.
///
/// Si no hay estaciones disponibles, se informa al usuario.
///
/// Si se proporciona una distancia máxima, solo se muestran las estaciones que estén dentro de esa distancia,
/// y si no hay ninguna estación cercana, se informa al usuario.
///
/// Si la cantidad de argumentos es incorrecta, se muestra un mensaje de error indicando el error y
/// el formato esperado para el comando.
pub async fn ejecutar_visualizar_estaciones(
    argumentos: Vec<&str>,
    app_usuario_addr: &mut Addr<UsuarioApp>,
) {
    if !validar_comando(&VISUALIZAR_ESTACIONES, &argumentos) {
        return;
    }

    let estaciones = match app_usuario_addr.send(VerEstacionesExistentes).await {
        Ok(estaciones) => estaciones,
        Err(_) => {
            eprintln!("\x1b[31mError al obtener estaciones disponibles.\x1b[0m");
            return;
        }
    };

    let distancia_maxima = argumentos
        .first()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(f64::MAX);

    if estaciones.is_empty() {
        println!("No hay estaciones disponibles en el sistema.");
    } else {
        let mut hay_estacion_cercana = false;
        for (id, nombre, coordenadas, distancia) in estaciones {
            if distancia > distancia_maxima {
                continue;
            }
            println!(
                "   ({}) {} {} (a {:.2}km)",
                id, nombre, coordenadas, distancia
            );
            hay_estacion_cercana = true;
        }
        if !hay_estacion_cercana {
            println!("No hay estaciones dentro de la distancia especificada.");
        }
    }
}

/// Función que ejecuta el comando para listar las bicicletas que el usuario tiene actualmente en uso,
/// mostrando su ID y la hora de inicio de uso.
///
/// Si no hay bicicletas en uso, se informa al usuario.
///
/// Si la cantidad de argumentos es incorrecta, se muestra un mensaje de error indicando el error y
/// el formato esperado para el comando.
pub async fn ejecutar_listar_bicicletas_en_uso(
    argumentos: Vec<&str>,
    app_usuario_addr: &mut Addr<UsuarioApp>,
) {
    if !validar_comando(&LISTAR_BICICLETAS_EN_USO, &argumentos) {
        return;
    }

    let bicicletas_en_uso = match app_usuario_addr.send(ListarBicicletasEnUso).await {
        Ok(bicicletas) => bicicletas,
        Err(_) => {
            eprintln!("\x1b[31mError al obtener bicicletas en uso.\x1b[0m");
            return;
        }
    };

    if bicicletas_en_uso.is_empty() {
        println!("No tenés bicicletas en uso actualmente.");
    } else {
        println!("Bicicletas en uso:");
        for bicicleta in bicicletas_en_uso {
            if let EstadoBicicleta::EnUso(inicio_de_uso, _) = bicicleta.estado {
                let hora_en_segundos = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Error al obtener el tiempo actual")
                    .as_secs()
                    // resto 3 horas para mostrar hora local y tiempo de uso para obtener hora de inicio de uso
                    .saturating_sub(10800 + inicio_de_uso.elapsed().as_secs());

                println!(
                    "   ID: {} - Se inició uso a las {:02}:{:02}",
                    bicicleta.id,
                    (hora_en_segundos / 3600) % 24,
                    (hora_en_segundos / 60) % 60
                );
            }
        }
    }
}

/// Función que ejecuta el comando para visualizar el estado de una o varias estaciones, mostrando su ID, nombre, coordenadas,
/// cantidad de slots libres y ocupados, y estado de conexión.
///
/// Si no hay estaciones conectadas en el sistema, se informa al usuario.
///
/// Si la cantidad de argumentos es incorrecta, se muestra un mensaje de error indicando el error y
/// el formato esperado para el comando.
pub async fn ejecutar_visualizar_estado_estaciones(
    argumentos: Vec<&str>,
    app_usuario_addr: &mut Addr<UsuarioApp>,
) {
    if !validar_comando(&VISUALIZAR_ESTADO_ESTACIONES, &argumentos) {
        return;
    }

    let ids_estaciones = argumentos
        .iter()
        .filter_map(|s| s.parse::<usize>().ok())
        .collect::<Vec<usize>>();

    let estaciones = match app_usuario_addr
        .send(SolicitarEstadoEstaciones { ids_estaciones })
        .await
    {
        Ok(estaciones) => estaciones,
        Err(_) => {
            eprintln!("\x1b[31mError al obtener estado de las estaciones.\x1b[0m");
            return;
        }
    };

    if let Some(estaciones) = estaciones {
        println!("Estado de las estaciones:");
        for (nombre, coordenadas, estacion) in estaciones {
            println!("  - Estación {}", estacion.id);
            println!("      Nombre: {}", nombre);
            println!("      Coordenadas: {}", coordenadas);
            println!("      Slots libres: {}", estacion.slots_libres);
            println!("      Slots ocupados: {}", estacion.slots_ocupados);
            println!(
                "      Estado: {:?}",
                match estacion.estado {
                    EstacionEstado::Conectada => "Conectada",
                    EstacionEstado::Incierto => "Incierto",
                }
            );
        }
    }
}

/// Función que ejecuta el comando para solicitar el estado de los slots de una estación específica,
/// mostrando los números de los slots libres y ocupados.
///
/// Si la estación se encuentra desconectada, se muestra un mensaje de error indicandolo.
///
/// Si la cantidad de argumentos es incorrecta, se muestra un mensaje de error indicando el error y
/// el formato esperado para el comando.
pub async fn ejecutar_solicitar_estado_estacion(
    argumentos: Vec<&str>,
    app_usuario_addr: &mut Addr<UsuarioApp>,
) {
    if !validar_comando(&SOLICITAR_ESTADO_ESTACION, &argumentos) {
        return;
    }

    let id_estacion = match argumentos[0].parse::<usize>() {
        Ok(id) => id,
        Err(_) => {
            eprintln!("\x1b[31mError: El ID de estación debe ser un número entero.\x1b[0m");
            return;
        }
    };

    let respuesta = match app_usuario_addr
        .send(SolicitarEstadoEstacion { id_estacion })
        .await
    {
        Ok(respuesta) => respuesta,
        Err(_) => {
            eprintln!(
                "Error al solicitar estado de la estación {}, se encuentra desconectada",
                id_estacion
            );
            return;
        }
    };

    match respuesta {
        Ok(respuesta) => {
            let estado_estacion = match EnviarEstado::from_bytes(&respuesta) {
                Ok(estado) => estado,
                Err(_) => {
                    eprintln!("\x1b[31mError al procesar respuesta de la estación.\x1b[0m");
                    return;
                }
            };
            println!("Estado de la estación:");
            println!("  Slots libres:");
            for slot in estado_estacion.slots_libres {
                println!("   - Slot {}", slot);
            }
            println!("  Slots ocupados:");
            for slot in estado_estacion.slots_ocupados {
                println!("   - Slot {}", slot);
            }
        }
        Err(_) => {
            eprintln!(
                "Error al solicitar estado de la estación {}, se encuentra desconectada",
                id_estacion
            );
        }
    }
}

/// Función que ejecuta el comando para iniciar el alquiler de una bicicleta, enviando un mensaje al actor
/// `UsuarioApp` con el ID de la estación y el número de slot donde se encuentra la bicicleta a alquilar.
///
/// Si la cantidad de argumentos es incorrecta, se muestra un mensaje de error indicando el error y
/// el formato esperado para el comando.
pub async fn ejecutar_iniciar_alquiler_bicicleta(
    argumentos: Vec<&str>,
    app_usuario_addr: &mut Addr<UsuarioApp>,
) {
    if !validar_comando(&INICIAR_ALQUILER_BICICLETA, &argumentos) {
        return;
    }

    let id_estacion = match argumentos[0].parse::<usize>() {
        Ok(id) => id,
        Err(_) => {
            eprintln!("\x1b[31mError: El ID de estación debe ser un número entero.\x1b[0m");
            return;
        }
    };
    let num_slot = match argumentos[1].parse::<u8>() {
        Ok(slot) => slot,
        Err(_) => {
            eprintln!("\x1b[31mError: El número de slot debe ser un número entero.\x1b[0m");
            return;
        }
    };

    match app_usuario_addr
        .send(IniciarAlquilerBicicleta {
            id_estacion,
            num_slot,
        })
        .await
    {
        Ok(_) => {}
        Err(_) => {
            eprintln!(
                "\x1b[31mError: El envío del mensaje para iniciar el alquiler de la bicicleta ha fallado.\x1b[0m"
            );
        }
    }
}

/// Función que ejecuta el comando para finalizar el alquiler de una bicicleta, enviando un mensaje al actor
/// `UsuarioApp` con el ID de la estación, el número de slot y el ID de la bicicleta que se desea finalizar el alquiler.
///
/// Si la cantidad de argumentos es incorrecta, se muestra un mensaje de error indicando el error y
/// el formato esperado para el comando.
pub async fn ejecutar_finalizar_alquiler_bicicleta(
    argumentos: Vec<&str>,
    app_usuario_addr: &mut Addr<UsuarioApp>,
) {
    if !validar_comando(&FINALIZAR_ALQUILER_BICICLETA, &argumentos) {
        return;
    }

    let id_estacion = match argumentos[0].parse::<usize>() {
        Ok(id) => id,
        Err(_) => {
            eprintln!("\x1b[31mError: El ID de estación debe ser un número entero.\x1b[0m");
            return;
        }
    };
    let num_slot = match argumentos[1].parse::<u8>() {
        Ok(slot) => slot,
        Err(_) => {
            eprintln!("\x1b[31mError: El número de slot debe ser un número entero.\x1b[0m");
            return;
        }
    };
    let id_bicicleta = match argumentos[2].parse::<usize>() {
        Ok(id) => id,
        Err(_) => {
            eprintln!("\x1b[31mError: El ID de bicicleta debe ser un número entero.\x1b[0m");
            return;
        }
    };

    match app_usuario_addr
        .send(FinalizarAlquilerBicicleta {
            id_estacion,
            num_slot,
            id_bicicleta,
        })
        .await
    {
        Ok(_) => {}
        Err(_) => {
            eprintln!(
                "\x1b[31mError: El envío del mensaje para finalizar el alquiler de la bicicleta ha fallado.\x1b[0m"
            );
        }
    }
}

/// Función que ejecuta el comando para actualizar las coordenadas actuales del usuario, enviando un mensaje al actor
/// `UsuarioApp` con las nuevas coordenadas.
///
/// Si la cantidad de argumentos es incorrecta, se muestra un mensaje de error indicando el error y
/// el formato esperado para el comando.
pub async fn ejecutar_actualizar_coordenadas(
    argumentos: Vec<&str>,
    app_usuario_addr: &mut Addr<UsuarioApp>,
) {
    if !validar_comando(&ACTUALIZAR_COORDENADAS, &argumentos) {
        return;
    }

    let latitud = match argumentos[0].parse::<isize>() {
        Ok(lat) => lat,
        Err(_) => {
            eprintln!("\x1b[31mError: La latitud debe ser un número entero.\x1b[0m");
            return;
        }
    };

    let longitud = match argumentos[1].parse::<isize>() {
        Ok(lon) => lon,
        Err(_) => {
            eprintln!("\x1b[31mError: La longitud debe ser un número entero.\x1b[0m");
            return;
        }
    };

    let coordenadas = Coordenadas::new(latitud, longitud);

    match app_usuario_addr
        .send(ActualizarCoordenadas { coordenadas })
        .await
    {
        Ok(_) => {
            println!("Coordenadas actualizadas correctamente.");
        }
        Err(_) => {
            eprintln!(
                "\x1b[31mError: El envío del mensaje para actualizar las coordenadas ha fallado.\x1b[0m"
            );
        }
    }
}

/// Función que ejecuta el comando para visualizar la información del usuario, enviando un mensaje al actor
/// `UsuarioApp` para solicitarla y luego mostrando el ID del usuario, sus coordenadas actuales, su tarjeta
/// de crédito y estado de conectividad.
///
/// Si no se encuentra información para el usuario, se muestra un mensaje de error indicandolo.
///
/// Si la cantidad de argumentos es incorrecta, se muestra un mensaje de error indicando el error y
/// el formato esperado para el comando.
pub async fn ejecutar_visualizar_info_usuario(
    argumentos: Vec<&str>,
    app_usuario_addr: &mut Addr<UsuarioApp>,
) {
    if !validar_comando(&VISUALIZAR_INFO_USUARIO, &argumentos) {
        return;
    }

    let respuesta = match app_usuario_addr.send(VisualizarInfoUsuario).await {
        Ok(respuesta) => respuesta,
        Err(_) => {
            eprintln!("\x1b[31mError al obtener información del usuario.\x1b[0m");
            return;
        }
    };

    let Some((id, coordenadas, tarjeta, conectado, cant_bicicletas)) = respuesta else {
        eprintln!("\x1b[31mError: No se encontró información para el usuario.\x1b[0m");
        return;
    };

    println!("Información del usuario:");
    println!("  ID: {}", id);
    println!("  Coordenadas actuales: {}", coordenadas);
    println!("  Tarjeta de crédito: {:?}", tarjeta);
    println!("  Conectado: {}", if conectado { "Sí" } else { "No" });
    println!("  Bicicletas en uso: {}", cant_bicicletas);
}

/// Función que ejecuta el comando para cambiar el estado de conectividad del usuario, enviando un mensaje al actor
/// `UsuarioApp` para realizar el cambio.
///
/// Si la cantidad de argumentos es incorrecta, se muestra un mensaje de error indicando el error y
/// el formato esperado para el comando.
pub async fn ejecutar_cambiar_conectividad(
    argumentos: Vec<&str>,
    app_usuario_addr: &mut Addr<UsuarioApp>,
) {
    if !validar_comando(&CAMBIAR_CONECTIVIDAD, &argumentos) {
        return;
    }

    match app_usuario_addr.send(CambiarConectividad).await {
        Ok(_) => {}
        Err(_) => {
            eprintln!(
                "\x1b[31mError: El envío del mensaje para cambiar el estado de conectividad ha fallado.\x1b[0m"
            );
        }
    }
}

/// Función que ejecuta el comando para salir de la aplicación, mostrando un mensaje de despedida y terminando el proceso.
///
/// Si la cantidad de argumentos es incorrecta, se muestra un mensaje de error indicando el error y
/// el formato esperado para el comando.
pub fn ejecutar_salir(argumentos: Vec<&str>) {
    if !validar_comando(&SALIR, &argumentos) {
        return;
    }

    println!("Saliendo del sistema...");
    std::process::exit(0);
}

/// Valida si la cantidad de argumentos proporcionados para un comando es adecuada, si es incorrecta muestra un mensaje de error
/// indicando el error y el formato esperado para el comando, además de retornar false. Si la cantidad de argumentos es correcta,
/// retorna true.
fn validar_comando(comando: &Comando, argumentos: &[&str]) -> bool {
    if !comando.validar_argumentos(argumentos) {
        let cantidad_min = comando.cantidad_argumentos_min();
        let cantidad_max = comando.cantidad_argumentos_max();
        let error;
        if cantidad_min == cantidad_max {
            error = format!(
                "Error: Comando '{}' requiere exactamente {} argumento(s). Formato esperado: '{}'",
                comando.numero, cantidad_min, comando.formato
            );
        } else if cantidad_max == usize::MAX {
            error = format!(
                "Error: Comando '{}' requiere al menos {} argumento(s). Formato esperado: '{}'",
                comando.numero, cantidad_min, comando.formato
            );
        } else {
            error = format!(
                "Error: Comando '{}' requiere entre {} y {} argumentos. Formato esperado: '{}'",
                comando.numero, cantidad_min, cantidad_max, comando.formato
            );
        }
        eprintln!("\x1b[31m{}\x1b[0m", error);

        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crear_comando_dummy(formato: Option<&'static str>) -> Comando {
        Comando {
            numero: "X",
            descripcion: "Comando de prueba",
            formato: formato.unwrap_or("X <arg1> [<arg2>] [<arg3> ...]"),
        }
    }

    #[test]
    fn test01_cantidad_argumentos_min() {
        let comando = crear_comando_dummy(Some("X <arg1> <arg2>"));
        assert_eq!(comando.cantidad_argumentos_min(), 2);
    }

    #[test]
    fn test02_cantidad_argumentos_min_con_opcional() {
        let comando = crear_comando_dummy(Some("X <arg1> [<arg2>]"));
        assert_eq!(comando.cantidad_argumentos_min(), 1);
    }

    #[test]
    fn test03_cantidad_argumentos_min_con_repetitivo() {
        let comando = crear_comando_dummy(Some("X <arg1> [<arg2> ... <arg3>]"));
        assert_eq!(comando.cantidad_argumentos_min(), 1);
    }

    #[test]
    fn test04_cantidad_argumentos_max() {
        let comando = crear_comando_dummy(Some("X <arg1> <arg2>"));
        assert_eq!(comando.cantidad_argumentos_max(), 2);
    }

    #[test]
    fn test05_cantidad_argumentos_max_con_opcional() {
        let comando = crear_comando_dummy(Some("X <arg1> [<arg2>]"));
        assert_eq!(comando.cantidad_argumentos_max(), 2);
    }

    #[test]
    fn test06_cantidad_argumentos_max_con_repetitivo() {
        let comando = crear_comando_dummy(Some("X <arg1> [<arg2> ... <arg3>]"));
        assert_eq!(comando.cantidad_argumentos_max(), usize::MAX);
    }

    // El impacto de ejecutar cada comando se testea en mensajes_internos.rs, donde se simula la interacción con las estaciones.
}
