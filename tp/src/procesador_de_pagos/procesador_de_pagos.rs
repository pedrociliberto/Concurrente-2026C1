//! procesador_de_pagos.rs
//!
//! Módulo principal del proceso del procesador de pagos. Contiene la lógica para escuchar conexiones entrantes de
//! las estaciones, procesar los mensajes recibidos y responder según el tipo de operación/solicitud.
//!

pub mod errores_procesador_de_pagos;

use core::str;
use std::{
    env,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread,
    time::Duration,
};
use tp::constantes::{ADDR_BASE, PUERTO_BASE_PROCESADOR_PAGOS};
use tp::objetos_bancarios::TarjetaDeCredito;

use crate::errores_procesador_de_pagos::ProcesadorDePagosError;

/// Punto de entrada principal para el procesador de pagos.
/// Lee los argumentos de línea de comandos para configurar la probabilidad de aceptación
/// de pagos, el tiempo de demora y comienza a escuchar conexiones entrantes de las
/// estaciones.
fn main() {
    let args: Vec<String> = env::args().collect();
    let proba = args
        .get(1)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(1.0);
    let sleep = args.get(2).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);

    if let Err(err) = escuchar_conexiones_entrantes(proba, sleep) {
        eprintln!("Error al escuchar conexiones entrantes: {:?}", err);
    }
}

/// Inicia un servidor TCP que escucha conexiones entrantes en el puerto configurado.
/// Por cada conexión recibida, se genera un nuevo hilo para manejarla.
///
/// # Parámetros
/// - `proba`: Probabilidad (entre 0.0 y 1.0) de que una preautorización sea aceptada.
/// - `sleep`: Cantidad de milisegundos que demora en procesarse cada operacióm.
///
/// # Retornos
/// - `Result<(), ProcesadorDePagosError>`: `Ok(())` si el servidor finaliza (teóricamente ciclo infinito), o un error si falla al enlazar el socket de escucha.
fn escuchar_conexiones_entrantes(proba: f32, sleep: u64) -> Result<(), ProcesadorDePagosError> {
    let listener = TcpListener::bind(
        format!("{ADDR_BASE}:{PUERTO_BASE_PROCESADOR_PAGOS}").parse::<SocketAddr>()?,
    )?;

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        println!("Nueva conexión entrante: {:?}", stream.peer_addr().unwrap());

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(sleep));
            if let Err(e) = manejar_conexion(stream, proba) {
                eprintln!("Error al manejar la conexión: {:?}", e);
            }
        });
    }

    Ok(())
}

/// Maneja la lectura y procesamiento de los mensajes recibidos a través de una conexión TCP.
/// Identifica el tipo de comando (preautorización, cobro de viaje o cobro de multa) y delega
/// su resolución a la función correspondiente.
///
/// # Parámetros
/// - `stream`: Flujo TCP de la conexión con el cliente (estación).
/// - `proba`: Probabilidad de que el pago de preautorización sea aceptado mediante generador de números aleatorios.
///
/// # Retornos
/// - `Result<(), ProcesadorDePagosError>`: `Ok(())` si el mensaje es procesado exitosamente o un error de lectura/parseo.
fn manejar_conexion(mut stream: TcpStream, proba: f32) -> Result<(), ProcesadorDePagosError> {
    let mut buffer = [0; 1024];
    let bytes_leidos = stream.read(&mut buffer)?;
    let mensaje = str::from_utf8(&buffer[..bytes_leidos])?;

    if mensaje.starts_with("PREPARE_PAGO_RETIRO") {
        println!("Pre-autorización recibida: {}", mensaje);
        let random = rand::random::<f32>();
        if random < proba {
            aceptar_preautorizacion(stream)?;
        } else {
            rechazar_preautorizacion(stream)?;
        }
    } else if mensaje.starts_with("COBRO_VIAJE") {
        aceptar_pago_viaje(stream, mensaje)?;
    } else if mensaje.starts_with("COBRO_MULTA") {
        cobrar_multa(stream, mensaje)?;
    } else {
        println!("Mensaje TCP no reconocido: {}", mensaje);
    }

    Ok(())
}

/// Envía un mensaje de confirmación (`COMMIT`) a través de la conexión TCP,
/// indicando que la preautorización del monto de seguridad fue exitosa.
///
/// # Parámetros
/// - `stream`: Flujo TCP activo hacia la estación que solicitó el pago.
///
/// # Retornos
/// - `Result<(), std::io::Error>`: `Ok(())` si los bytes se enviaron correctamente o un error de red.
fn aceptar_preautorizacion(mut stream: TcpStream) -> Result<(), std::io::Error> {
    stream.write_all("COMMIT\n".as_bytes())?;
    println!("Pre-autorización aceptada.");
    Ok(())
}

/// Envía un mensaje de rechazo (`ABORT`) a través de la conexión TCP,
/// indicando que la preautorización falló (por ejemplo, fondos insuficientes simulados por probabilidad).
///
/// # Parámetros
/// - `stream`: Flujo TCP activo hacia la estación.
///
/// # Retornos
/// - `Result<(), std::io::Error>`: `Ok(())` si los bytes se enviaron correctamente o un error de red.
fn rechazar_preautorizacion(mut stream: TcpStream) -> Result<(), std::io::Error> {
    stream.write_all("ABORT\n".as_bytes())?;
    println!("Pre-autorización rechazada.");
    Ok(())
}

/// Procesa y asienta el cobro definitivo de un viaje. Extrae del mensaje el monto del viaje,
/// el monto de seguridad a reintegrar y los datos de la tarjeta para su simulación,
/// y luego envía la confirmación de la operación (`PAGO_VIAJE_ACEPTADO`).
///
/// # Parámetros
/// - `stream`: Flujo TCP activo.
/// - `mensaje`: Cadena de texto recibida conteniendo los datos del cobro a procesar.
///
/// # Retornos
/// - `Result<(), ProcesadorDePagosError>`: `Ok(())` en caso de éxito, o error si los parámetros numéricos no pueden ser parseados.
fn aceptar_pago_viaje(mut stream: TcpStream, mensaje: &str) -> Result<(), ProcesadorDePagosError> {
    println!("Cobro de viaje recibido: {}", mensaje);

    let partes: Vec<&str> = mensaje.split(':').collect();
    let monto_viaje = partes[1].parse::<usize>()?;
    let monto_de_seguridad = partes[2].parse::<usize>()?;
    let tarjeta = TarjetaDeCredito::new(
        &partes[4][2..18],
        partes[5][1..4].parse()?,
        &partes[6][2..7],
    );

    println!(
        "Se realiza cobro de viaje por monto {} con tarjeta {:?}",
        monto_viaje, tarjeta
    );
    println!(
        "Se devuelve monto de seguridad equivalente a {} a la tarjeta {:?}",
        monto_de_seguridad, tarjeta
    );

    stream.write_all("PAGO_VIAJE_ACEPTADO\n".as_bytes())?;
    println!("Pago de viaje aceptado.");
    Ok(())
}

/// Procesa y asienta el cobro de una multa (por ejemplo, por no devolver la bicicleta a tiempo antes de ser declarada robada).
/// Extrae los datos del mensaje, registra la operación en la consola y devuelve la confirmación (`MULTA_ACEPTADA`).
///
/// # Parámetros
/// - `stream`: Flujo TCP activo.
/// - `mensaje`: Cadena de texto recibida conteniendo el ID del usuario infractor y tarjeta asociada.
///
/// # Retornos
/// - `Result<(), std::io::Error>`: `Ok(())` si la escritura en el flujo fue exitosa.
fn cobrar_multa(mut stream: TcpStream, mensaje: &str) -> Result<(), std::io::Error> {
    let partes: Vec<&str> = mensaje.split(':').collect();
    let id_usuario = partes[1];
    let tarjeta = &partes[3][2..18];
    println!(
        "Aplicando multa por bicicleta robada al usuario {} con tarjeta de número {}.",
        id_usuario, tarjeta
    );
    stream.write_all("MULTA_ACEPTADA\n".as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // Función auxiliar para instanciar rápidamente conexiones TCP (Cliente y Servidor) para los tests
    fn setup_test_connection() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let client = TcpStream::connect(addr).unwrap();
        let (server, _) = listener.accept().unwrap();

        // Configuramos un timeout razonable para evitar que las pruebas se cuelguen si algo sale mal
        client
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();

        (client, server)
    }

    #[test]
    fn test01_preautorizacion_aceptada_con_proba_alta() {
        let (mut client, server) = setup_test_connection();

        client.write_all(b"PREPARE_PAGO_RETIRO:1:1:100:TarjetaDeCredito { numero: \"1234567890123456\", cod_seguridad: 123, vencimiento: \"12/25\" }").unwrap();

        // Con proba 1.0, el pago SIEMPRE debe ser aceptado
        manejar_conexion(server, 1.0).unwrap();

        let mut buf = [0; 128];
        let n = client.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"COMMIT\n");
    }

    #[test]
    fn test02_preautorizacion_rechazada_con_proba_nula() {
        let (mut client, server) = setup_test_connection();

        client
            .write_all(b"PREPARE_PAGO_RETIRO:1:1:100:TarjetaDeCredito...")
            .unwrap();

        // Con proba 0.0, el pago SIEMPRE debe ser rechazado
        manejar_conexion(server, 0.0).unwrap();

        let mut buf = [0; 128];
        let n = client.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"ABORT\n");
    }

    #[test]
    fn test03_cobro_viaje_exitoso_y_parseo_de_tarjeta() {
        let (mut client, server) = setup_test_connection();

        // Formato exacto que la Estación envía por red con la representación `Debug` de TarjetaDeCredito
        let msg = "COBRO_VIAJE:1500:100:TarjetaDeCredito { numero: \"1234567890123456\", cod_seguridad: 123, vencimiento: \"12/25\" }";
        client.write_all(msg.as_bytes()).unwrap();

        manejar_conexion(server, 1.0).expect("Fallo durante el parseo de la cadena de pago.");

        let mut buf = [0; 128];
        let n = client.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"PAGO_VIAJE_ACEPTADO\n");
    }

    #[test]
    fn test04_cobro_multa_aceptado() {
        let (mut client, server) = setup_test_connection();

        client
            .write_all(b"COBRO_MULTA:42:TarjetaDeCredito { numero: \"1234567890123456\", cod_seguridad: 123, vencimiento: \"12/25\" }")
            .unwrap();
        manejar_conexion(server, 1.0).unwrap();

        let mut buf = [0; 128];
        let n = client.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"MULTA_ACEPTADA\n");
    }

    #[test]
    fn test05_mensaje_desconocido_no_rompe_procesador() {
        let (mut client, server) = setup_test_connection();

        client.write_all(b"MENSAJE_BASURA:999:xyz").unwrap();

        let result = manejar_conexion(server, 1.0);
        assert!(result.is_ok());

        // Como se envía nada como respuesta para mensajes no conocidos, el cliente leerá un EOF al cerrarse la conexión
        let mut buf = [0; 128];
        let n = client.read(&mut buf).unwrap_or(0);
        assert_eq!(n, 0);
    }
}
