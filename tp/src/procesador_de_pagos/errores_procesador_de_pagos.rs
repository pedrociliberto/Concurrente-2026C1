//! errores_procesador_de_pagos.rs
//!
//! Módulo que define los errores específicos que pueden ocurrir durante la ejecución del proceso del procesador de pagos.
//!

use std::{io::Error, num::ParseIntError, str::Utf8Error};

/// Enumeración que engloba todos los posibles errores que pueden ocurrir en el subsistema del procesador de pagos.
#[derive(Debug)]
pub enum ProcesadorDePagosError {
    /// Envuelve errores estándar de Entrada/Salida (`std::io::Error`), aplicable a lectura de archivos o flujos de red.
    Io(Error),
    /// Ocurre cuando una dirección IP o puerto tiene un formato inválido o no puede ser parseado.
    InvalidAddress(String),
    /// Envuelve errores estándar de parseo de enteros (`ParseIntError`).
    ParseInt(ParseIntError),
    /// Envuelve errores estándar de parseo de cadenas UTF-8 (`Utf8Error`).
    Utf8(Utf8Error),
}

impl std::fmt::Display for ProcesadorDePagosError {
    /// Formatea el error para generar un mensaje descriptivo que sea fácil de entender en los logs del sistema.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "Error de Entrada/Salida: {e}"),
            Self::InvalidAddress(addr) => write!(f, "Dirección IP o puerto inválido: {addr}"),
            Self::ParseInt(e) => write!(f, "Error al parsear enteros: {e}"),
            Self::Utf8(e) => write!(f, "Error al parsear cadenas UTF-8: {e}"),
        }
    }
}

impl From<std::io::Error> for ProcesadorDePagosError {
    /// Convierte de forma automática un error de Entrada/Salida estándar al tipo general `ProcesadorDePagosError`.
    fn from(err: std::io::Error) -> Self {
        ProcesadorDePagosError::Io(err)
    }
}

impl From<std::net::AddrParseError> for ProcesadorDePagosError {
    /// Convierte de forma automática un error de parseo de direcciones IP al tipo general `ProcesadorDePagosError`.
    fn from(err: std::net::AddrParseError) -> Self {
        ProcesadorDePagosError::InvalidAddress(err.to_string())
    }
}

impl From<ParseIntError> for ProcesadorDePagosError {
    /// Convierte de forma automática un error de parseo de enteros al tipo general `ProcesadorDePagosError`.
    fn from(err: ParseIntError) -> Self {
        ProcesadorDePagosError::ParseInt(err)
    }
}

impl From<Utf8Error> for ProcesadorDePagosError {
    /// Convierte de forma automática un error de parseo de cadenas UTF-8 al tipo general `ProcesadorDePagosError`.
    fn from(err: Utf8Error) -> Self {
        ProcesadorDePagosError::Utf8(err)
    }
}
