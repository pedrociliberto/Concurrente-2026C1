//! usuario_app.rs
//!
//! Este módulo es el punto de entrada de la aplicación del usuario. Aquí se define la función `main`,
//! que se encarga de iniciar el sistema Actix, crear el actor de la aplicación del usuario, y
//! manejar la interacción con el usuario a través de la consola.
//!

pub mod actor;
pub mod comandos;
pub mod config;
pub mod mensajes_internos;

use crate::config::crear_actor_usuario_app;
use actix::{Actor, Addr, System};
use actor::UsuarioApp;
use comandos::*;
use std::{env, io::Write};

const FORMATO_CMD: &str = "cargo run --bin usuario_app -- <id_usuario> [<longitud> <latitud>]";

/// Punto de entrada principal para el proceso de la aplicación del usuario. Se encarga crear el
/// actor `UsuarioApp` a partir de lo recibido por línea de comandos o según el estado previamente
/// alamcenado para el usuario con el id indicado. Luego, comienza un loop para recibir comandos del
/// usuario a través de la consola y ejecutas las acciones correspondientes a cada comando.
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 && args.len() != 4 {
        eprintln!("Error: Cantidad de argumentos incorrecta.");
        eprintln!("Uso: {}", FORMATO_CMD);
        std::process::exit(1);
    }

    let id = args
        .get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    let longitud = args.get(2).map(|s| s.parse::<isize>().ok()).unwrap_or(None);
    let latitud = args.get(3).map(|s| s.parse::<isize>().ok()).unwrap_or(None);

    println!("\n¡Bienvenido al sistema de bicicletas de la ciudad!");

    let sistema_actix = System::new();
    sistema_actix.block_on(async {
        let mut app_usuario_addr = crear_actor_usuario_app(id, longitud, latitud).start();

        imprimir_comandos();
        loop {
            if let Ok(respuesta_usuario) = obtener_respuesta_usuario() {
                ejecutar_comando(respuesta_usuario, &mut app_usuario_addr).await;
            }
        }
    });

    if sistema_actix.run().is_err() {
        eprintln!("Error al ejecutar el sistema Actix.");
    }
}

/// Imprime en la consola la lista de comandos disponibles para el usuario mostrando el número, formato y descripción
/// de cada comando.
fn imprimir_comandos() {
    println!("\n\x1b[1;33mSeleccioná un comando (indicando su número y argumentos):\x1b[0m");
    println!(
        "  \x1b[1;32m{:<38}\x1b[0m | \x1b[36mDescripción\x1b[0m",
        "Comando"
    );
    for comando in COMANDOS.iter() {
        println!(
            "  \x1b[1;32m{:<38}\x1b[0m | \x1b[36m{}\x1b[0m",
            comando.formato, comando.descripcion
        );
    }
    println!();
}

/// Obtiene la respuesta del usuario a través de la entrada estándar. Se muestra un mensaje solicitando el comando, y
/// luego se lee la línea ingresada por el usuario, retornando la respuesta como una cadena de texto sin espacios al
/// inicio o al final.
fn obtener_respuesta_usuario() -> Result<String, std::io::Error> {
    print!("\nComando: ");

    std::io::stdout().flush()?;
    let mut respuesta = String::new();
    std::io::stdin().read_line(&mut respuesta)?;

    println!();

    Ok(respuesta.trim().to_string())
}

/// Ejecuta el comando ingresado por el usuario, identificando su número y sus argumentos,
/// y llamando a la función correspondiente para ejecutar la acción a este. Cada una de estas
/// funciones se encarga de validar la cantidad de argumentos, mostrar mensajes de error si es necesario,
/// e imprimir los resultados correspondientes.
///
/// Si el número del comando no es válido, se muestra un mensaje de error.
async fn ejecutar_comando(respuesta: String, app_usuario_addr: &mut Addr<UsuarioApp>) {
    let mut respuesta = respuesta.split_whitespace();
    let numero_comando = respuesta.next().unwrap_or("");
    let argumentos = respuesta.collect::<Vec<&str>>();

    if numero_comando == IMPRIMIR_AYUDA.numero {
        imprimir_comandos();
    } else if numero_comando == VISUALIZAR_ESTACIONES.numero {
        ejecutar_visualizar_estaciones(argumentos, app_usuario_addr).await;
    } else if numero_comando == LISTAR_BICICLETAS_EN_USO.numero {
        ejecutar_listar_bicicletas_en_uso(argumentos, app_usuario_addr).await;
    } else if numero_comando == VISUALIZAR_ESTADO_ESTACIONES.numero {
        ejecutar_visualizar_estado_estaciones(argumentos, app_usuario_addr).await;
    } else if numero_comando == SOLICITAR_ESTADO_ESTACION.numero {
        ejecutar_solicitar_estado_estacion(argumentos, app_usuario_addr).await;
    } else if numero_comando == INICIAR_ALQUILER_BICICLETA.numero {
        ejecutar_iniciar_alquiler_bicicleta(argumentos, app_usuario_addr).await;
    } else if numero_comando == FINALIZAR_ALQUILER_BICICLETA.numero {
        ejecutar_finalizar_alquiler_bicicleta(argumentos, app_usuario_addr).await;
    } else if numero_comando == ACTUALIZAR_COORDENADAS.numero {
        ejecutar_actualizar_coordenadas(argumentos, app_usuario_addr).await;
    } else if numero_comando == VISUALIZAR_INFO_USUARIO.numero {
        ejecutar_visualizar_info_usuario(argumentos, app_usuario_addr).await;
    } else if numero_comando == CAMBIAR_CONECTIVIDAD.numero {
        ejecutar_cambiar_conectividad(argumentos, app_usuario_addr).await;
    } else if numero_comando == SALIR.numero {
        ejecutar_salir(argumentos);
    } else {
        println!("\x1b[31mError: Comando no válido. Por favor, intente nuevamente.\x1b[0m");
        println!(
            "Ingrese '{}' para ver la lista de comandos disponibles.",
            IMPRIMIR_AYUDA.numero
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix::Actor;
    use std::collections::HashMap;
    use tp::{
        constantes::{ADDR_BASE, PUERTO_BASE_ESTACION},
        coordenadas::Coordenadas,
        objetos_bancarios::TarjetaDeCredito,
    };

    fn crear_usuario_app_dummy() -> UsuarioApp {
        let mut estaciones = HashMap::new();
        estaciones.insert(
            1,
            (
                "Estación 1".to_string(),
                Coordenadas::new(0, 0),
                format!("{}:{}", ADDR_BASE, PUERTO_BASE_ESTACION + 1)
                    .parse()
                    .unwrap(),
            ),
        );
        estaciones.insert(
            2,
            (
                "Estación 2".to_string(),
                Coordenadas::new(1, 1),
                format!("{}:{}", ADDR_BASE, PUERTO_BASE_ESTACION + 2)
                    .parse()
                    .unwrap(),
            ),
        );
        estaciones.insert(
            3,
            (
                "Estación 3".to_string(),
                Coordenadas::new(2, 2),
                format!("{}:{}", ADDR_BASE, PUERTO_BASE_ESTACION + 3)
                    .parse()
                    .unwrap(),
            ),
        );
        UsuarioApp::new(
            1,
            Coordenadas::new(0, 0),
            Some(1),
            TarjetaDeCredito::new("1234567890123456", 123, "12/25"),
            estaciones,
            HashMap::new(),
        )
    }

    #[actix::test]
    async fn test01_ejecutar_comando_desconocido_no_hace_panic() {
        let respuesta = "999 arg1 arg2".to_string();
        let mut app_usuario_addr = crear_usuario_app_dummy().start();
        ejecutar_comando(respuesta, &mut app_usuario_addr).await;
        assert!(app_usuario_addr.connected());
    }

    #[actix::test]
    async fn test02_ejecutar_comando_con_argumentos_de_mas_no_hace_panic() {
        let respuesta = format!("{} arg1 arg2 arg3", IMPRIMIR_AYUDA.numero);
        let mut app_usuario_addr = crear_usuario_app_dummy().start();
        ejecutar_comando(respuesta, &mut app_usuario_addr).await;
        assert!(app_usuario_addr.connected());
    }

    #[actix::test]
    async fn test03_ejecutar_comando_con_argumentos_de_menos_no_hace_panic() {
        let respuesta = SOLICITAR_ESTADO_ESTACION.numero.to_string();
        let mut app_usuario_addr = crear_usuario_app_dummy().start();
        ejecutar_comando(respuesta, &mut app_usuario_addr).await;
        assert!(app_usuario_addr.connected());
    }

    // El impacto de ejecutar cada comando se testea en mensajes_internos.rs, donde se simula la interacción con las estaciones.
}
