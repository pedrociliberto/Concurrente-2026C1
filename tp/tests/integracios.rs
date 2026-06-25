use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use std::{fs, thread};

use tp::constantes::{ADDR_BASE, PUERTO_BASE_ESTACION};
use tp::msjs_app_usuario_estacion::{
    BicicletaDevueltaCorrectamente, DevolverBicicleta, EntregarBicicleta,
    HayPedidoEnProcesoEnEseSlot, NoTengoBicicletaEnEseSlot, PedirBicicleta,
};
use tp::objetos_bancarios::TarjetaDeCredito;

/// Estructura de guarda para limpiar los procesos hijos cuando finalice o falle el test.
struct DropeadorDeProceso(Child);

impl Drop for DropeadorDeProceso {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

// Funciones auxiliares para escribir y leer mensajes TCP.
fn enviar_mensaje(stream: &mut TcpStream, mensaje: &[u8]) {
    let len = mensaje.len() as u32;
    stream.write_all(&len.to_be_bytes()).unwrap();
    stream.write_all(mensaje).unwrap();
    stream.flush().unwrap();
}

fn recibir_mensaje(stream: &mut TcpStream) -> Vec<u8> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .expect("Error al leer prefijo de longitud TCP");
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buffer = vec![0u8; len];
    stream
        .read_exact(&mut buffer)
        .expect("Error al leer payload de bytes TCP");
    buffer
}

#[test]
fn test01_flujo_alquilar_y_devolver_bicicleta() {
    let id_estacion = 11;
    let archivo_estado = format!("./src/estado_estaciones/estacion_{}.state", id_estacion);

    // 1. Se inicia el procesador de pagos como proceso independiente.
    let bin_procesador = env!("CARGO_BIN_EXE_procesador_de_pagos");
    let procesador = Command::new(bin_procesador)
        .args(["1.0", "0"]) // proba = 1.0 (siempre acepta), sleep = 0ms
        .stdout(Stdio::null())
        .spawn()
        .expect("Fallo al iniciar el binario de procesador de pagos");
    let _guard_procesador = DropeadorDeProceso(procesador);

    // 2. Se inicia una estación como proceso independiente.
    let bin_estacion = env!("CARGO_BIN_EXE_estacion");
    let estacion = Command::new(bin_estacion)
        .args([&id_estacion.to_string(), "1"]) // conectado = 1 (true)
        .stdout(Stdio::null())
        .spawn()
        .expect("Fallo al iniciar el binario de estación");
    let _guard_estacion = DropeadorDeProceso(estacion);

    thread::sleep(Duration::from_secs(4));

    let puerto_estacion = PUERTO_BASE_ESTACION + id_estacion as u16;
    let tarjeta = TarjetaDeCredito::new("1234567890123456", 123, "12/25");

    let mut bicicleta_alquilada = None;
    let mut slot_exitoso = 1;

    // 3. Se itera por los slots hasta conseguir alquilar con éxito la primera bicicleta disponible.
    for slot in 1..=20 {
        let addr = format!("{}:{}", ADDR_BASE, puerto_estacion);
        if let Ok(mut stream) = TcpStream::connect(&addr) {
            let pedir_msg = PedirBicicleta {
                id: 10,
                numero_slot: slot,
                tarjeta_de_credito: tarjeta.clone(),
            };

            enviar_mensaje(&mut stream, &pedir_msg.as_bytes());

            let respuesta_bytes = recibir_mensaje(&mut stream);
            if let Ok(entregar_bici) = EntregarBicicleta::from_bytes(&respuesta_bytes) {
                bicicleta_alquilada = Some(entregar_bici.bicicleta);
                slot_exitoso = slot;
                break;
            }
        }
    }

    let bicicleta_alquilada = bicicleta_alquilada
        .expect("No se encontró ninguna bicicleta para alquilar (slots 1 a 20).");

    // Simula un breve tiempo de uso del servicio.
    thread::sleep(Duration::from_secs(1));

    // 4. Se devuelve la bicicleta exacta al mismo slot, que ahora se encuentra liberado.
    let addr = format!("{}:{}", ADDR_BASE, puerto_estacion);
    let mut stream =
        TcpStream::connect(&addr).expect("Fallo al conectarse para devolver bicicleta");

    let devolver_msg = DevolverBicicleta {
        id: 10,
        numero_slot: slot_exitoso,
        tarjeta_de_credito: tarjeta,
        bicicleta: bicicleta_alquilada,
    };
    enviar_mensaje(&mut stream, &devolver_msg.as_bytes());

    let respuesta_devolucion_bytes = recibir_mensaje(&mut stream);
    let devuelta_ok = BicicletaDevueltaCorrectamente::from_bytes(&respuesta_devolucion_bytes);
    assert!(devuelta_ok.is_ok(), "La devolución falló o fue rechazada.");
    fs::remove_file(archivo_estado).unwrap();
}

#[test]
fn test02_flujo_luego_de_alquilar_una_bicicleta_el_slot_queda_vacio() {
    let id_estacion = 11;
    let archivo_estado = format!("./src/estado_estaciones/estacion_{}.state", id_estacion);

    // 1. Iniciar procesos
    let bin_procesador = env!("CARGO_BIN_EXE_procesador_de_pagos");
    let procesador = Command::new(bin_procesador)
        .args(["1.0", "0"])
        .stdout(Stdio::null())
        .spawn()
        .expect("Fallo al iniciar el binario de procesador de pagos");
    let _guard_procesador = DropeadorDeProceso(procesador);

    let bin_estacion = env!("CARGO_BIN_EXE_estacion");
    let estacion = Command::new(bin_estacion)
        .args([&id_estacion.to_string(), "1"])
        .stdout(Stdio::null())
        .spawn()
        .expect("Fallo al iniciar el binario de estación");
    let _guard_estacion = DropeadorDeProceso(estacion);

    thread::sleep(Duration::from_secs(4));

    let puerto_estacion = PUERTO_BASE_ESTACION + id_estacion as u16;
    let tarjeta = TarjetaDeCredito::new("1111222233334444", 456, "11/26");

    let mut slot_exitoso = 0;

    for slot in 1..=20 {
        let addr = format!("{}:{}", ADDR_BASE, puerto_estacion);
        if let Ok(mut stream) = TcpStream::connect(&addr) {
            let pedir_msg = PedirBicicleta {
                id: 20,
                numero_slot: slot,
                tarjeta_de_credito: tarjeta.clone(),
            };
            enviar_mensaje(&mut stream, &pedir_msg.as_bytes());

            let respuesta_bytes = recibir_mensaje(&mut stream);
            if EntregarBicicleta::from_bytes(&respuesta_bytes).is_ok() {
                slot_exitoso = slot;
                break;
            }
        }
    }

    assert_ne!(
        slot_exitoso, 0,
        "No se pudo alquilar ninguna bicicleta para iniciar el test."
    );

    let addr = format!("{}:{}", ADDR_BASE, puerto_estacion);
    let mut stream_segundo_intento =
        TcpStream::connect(&addr).expect("Fallo al conectar por segunda vez");

    let pedir_msg_2 = PedirBicicleta {
        id: 21,
        numero_slot: slot_exitoso,
        tarjeta_de_credito: tarjeta.clone(),
    };
    enviar_mensaje(&mut stream_segundo_intento, &pedir_msg_2.as_bytes());

    let respuesta_bytes_2 = recibir_mensaje(&mut stream_segundo_intento);

    let no_hay_bici = NoTengoBicicletaEnEseSlot::from_bytes(&respuesta_bytes_2);
    assert!(
        no_hay_bici.is_ok(),
        "La respuesta no fue 'NoTengoBicicletaEnEseSlot'. Respuesta recibida: {:?}",
        respuesta_bytes_2
    );
    assert_eq!(
        no_hay_bici.unwrap().numero_slot,
        slot_exitoso,
        "El número de slot en el mensaje de error no coincide."
    );
    fs::remove_file(archivo_estado).unwrap();
}

#[test]
fn test03_no_se_puede_alquilar_mientras_el_slot_se_encuentra_en_proceso_de_preautorizacion() {
    let id_estacion = 11;
    let archivo_estado = format!("./src/estado_estaciones/estacion_{}.state", id_estacion);

    let bin_procesador = env!("CARGO_BIN_EXE_procesador_de_pagos");
    let procesador = Command::new(bin_procesador)
        .args(["1.0", "2000"])
        .stdout(Stdio::null())
        .spawn()
        .expect("Fallo al iniciar el binario de procesador de pagos");
    let _guard_procesador = DropeadorDeProceso(procesador);

    let bin_estacion = env!("CARGO_BIN_EXE_estacion");
    let estacion = Command::new(bin_estacion)
        .args([&id_estacion.to_string(), "1"]) // conectado = true
        .stdout(Stdio::null())
        .spawn()
        .expect("Fallo al iniciar el binario de estación");
    let _guard_estacion = DropeadorDeProceso(estacion);

    thread::sleep(Duration::from_secs(4));

    let puerto_estacion = PUERTO_BASE_ESTACION + id_estacion as u16;
    let tarjeta1 = TarjetaDeCredito::new("1111111111111111", 111, "01/27");
    let tarjeta2 = TarjetaDeCredito::new("2222222222222222", 222, "02/28");

    let slot_a_probar = 8;
    let addr = format!("{}:{}", ADDR_BASE, puerto_estacion);

    let mut stream_usuario1 = TcpStream::connect(&addr).expect("Fallo al conectar usuario 1");
    let pedir_msg1 = PedirBicicleta {
        id: 30,
        numero_slot: slot_a_probar,
        tarjeta_de_credito: tarjeta1,
    };
    enviar_mensaje(&mut stream_usuario1, &pedir_msg1.as_bytes());

    thread::sleep(Duration::from_millis(500));

    let mut stream_usuario2 = TcpStream::connect(&addr).expect("Fallo al conectar usuario 2");
    let pedir_msg2 = PedirBicicleta {
        id: 31,
        numero_slot: slot_a_probar,
        tarjeta_de_credito: tarjeta2,
    };
    enviar_mensaje(&mut stream_usuario2, &pedir_msg2.as_bytes());

    let respuesta_usuario2_bytes = recibir_mensaje(&mut stream_usuario2);
    let en_proceso = HayPedidoEnProcesoEnEseSlot::from_bytes(&respuesta_usuario2_bytes);

    assert!(
        en_proceso.is_ok(),
        "La respuesta para el usuario 2 no fue 'HayPedidoEnProcesoEnEseSlot'. Respuesta: {:?}",
        respuesta_usuario2_bytes
    );
    assert_eq!(
        en_proceso.unwrap().numero_slot,
        slot_a_probar,
        "El slot en el mensaje de error para el usuario 2 no coincide."
    );

    let respuesta_usuario1_bytes = recibir_mensaje(&mut stream_usuario1);
    assert!(
        EntregarBicicleta::from_bytes(&respuesta_usuario1_bytes).is_ok(),
        "El usuario 1 no recibió la confirmación de su alquiler después del delay."
    );
    fs::remove_file(archivo_estado).unwrap();
}

/*
#[test]
fn test04_se_multa_a_usuario_que_roba_bicicleta() {
    use std::io::{BufRead, BufReader};

    let id_estacion = 11;
    let id_usuario = 42;
    let archivo_estado = format!("./src/estado_estaciones/estacion_{}.state", id_estacion);

    let bin_procesador = env!("CARGO_BIN_EXE_procesador_de_pagos");
    let mut procesador = Command::new(bin_procesador)
        .args(["1.0", "0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Fallo al iniciar el binario de procesador de pagos");

    let procesador_stdout = procesador
        .stdout
        .take()
        .expect("No se pudo capturar stdout del procesador");
    let _guard_procesador = ProcesoGuard(procesador);

    let bin_estacion = env!("CARGO_BIN_EXE_estacion");
    let estacion = Command::new(bin_estacion)
        .args([&id_estacion.to_string(), "1"]) // conectado = true
        .stdout(Stdio::null())
        .spawn()
        .expect("Fallo al iniciar el binario de estación");
    let _guard_estacion = ProcesoGuard(estacion);

    thread::sleep(Duration::from_secs(4));

    let puerto_estacion = PUERTO_BASE_ESTACION + id_estacion as u16;
    let tarjeta = TarjetaDeCredito::new("4242424242424242", 424, "04/24");
    let mut slot_exitoso = 0;

    for slot in 1..=20 {
        let addr = format!("{}:{}", ADDR_BASE, puerto_estacion);
        if let Ok(mut stream) = TcpStream::connect(&addr) {
            let pedir_msg = PedirBicicleta { id: id_usuario, numero_slot: slot, tarjeta_de_credito: tarjeta.clone() };
            enviar_mensaje(&mut stream, &pedir_msg.as_bytes());

            let respuesta_bytes = recibir_mensaje(&mut stream);
            if EntregarBicicleta::from_bytes(&respuesta_bytes).is_ok() {
                slot_exitoso = slot;
                break;
            }
        }
    }
    assert_ne!(slot_exitoso, 0, "No se pudo alquilar ninguna bicicleta para el test de multa.");

    let tiempo_espera = tp::constantes::TIEMPO_MAX_PRE_ROBO + 2;
    thread::sleep(Duration::from_secs(tiempo_espera));

    let reader = BufReader::new(procesador_stdout);
    let output = reader.lines().map(|l| l.unwrap_or_default()).collect::<String>();

    assert!(output.contains(&format!("Aplicando multa por bicicleta robada al usuario {}", id_usuario)),
        "No se detectó el mensaje de multa en el stdout del procesador de pagos. Output: {}", output);
    fs::remove_file(archivo_estado).unwrap();
}
*/
