use std::time::Duration;

pub const ADDR_BASE: &str = "127.0.0.1";
pub const PUERTO_BASE_ESTACION: u16 = 8000;
pub const PUERTO_BASE_APP_USUARIO: u16 = 9000;
pub const PUERTO_BASE_ELECCION: u16 = 8100;
pub const PUERTO_BASE_SINCRO_TCP: u16 = 8200;
pub const PUERTO_BASE_PROCESADOR_PAGOS: u16 = 10000;

pub const CANTIDAD_ESTACIONES: usize = 11;

pub const TIMEOUT_UDP: Duration = Duration::from_millis(200);
pub const TIEMPO_MAX_PRE_ROBO: u64 = 10; // segundos
