/* ssl.rs
 *                            _ _       _
 *                           | (_)     | |
 *  _ __ ___   ___  ___  __ _| |_ _ __ | | __
 * | '_ ` _ \ / _ \/ __|/ _` | | | '_ \| |/ /
 * | | | | | |  __/\__ \ (_| | | | | | |   <
 * |_| |_| |_|\___||___/\__,_|_|_|_| |_|_|\_\
 *
 * Copyright (C) 2017 Baidu USA.
 *
 * This file is part of Mesalink.
 */

use std::sync::Arc;
use std::net::TcpStream;
use std::io::{Read, Write};
use std::ffi::CStr;
use std::os::unix::io::FromRawFd;
use std::slice;
use std::ptr;
use libc::{c_char, c_int, c_uchar};
use rustls;
use rustls::{Session, Stream};
use webpki_roots::TLS_SERVER_ROOTS;
use ssl::err::{mesalink_push_error, ErrorCode};

const MAGIC: u32 = 0xc0d4c5a9;

#[repr(C)]
pub struct MESALINK_METHOD {
    magic: u32,
    tls_version: rustls::ProtocolVersion,
}

#[repr(C)]
pub struct MESALINK_CTX {
    magic: u32,
    client_config: Arc<rustls::ClientConfig>,
    server_config: Arc<rustls::ServerConfig>,
}

#[repr(C)]
pub struct MESALINK_SSL<'a, S: 'a + Session, T: 'a + Read + Write> {
    magic: u32,
    context: &'a mut MESALINK_CTX,
    hostname: Option<&'a CStr>,
    socket: Option<TcpStream>,
    session: Option<S>,
    stream: Option<Stream<'a, S, T>>,
}

pub enum SslConstants {
    SslFailure = 0,
    SslSuccess = 1,
}

macro_rules! sanitize_ptr_return_null {
    ( $ptr_var:ident ) => {
        if $ptr_var.is_null() {
            return ptr::null_mut();
        }
        let obj = unsafe { &* $ptr_var };
        if obj.magic != MAGIC {
            return ptr::null_mut();
        }
    }
}

macro_rules! sanitize_ptr_return_fail {
    ( $ptr_var:ident ) => {
        if $ptr_var.is_null() {
            return SslConstants::SslFailure as c_int;
        }
        let obj = unsafe { &*$ptr_var };
        if obj.magic != MAGIC {
            return SslConstants::SslFailure as c_int;
        }
    }
}

#[no_mangle]
pub extern "C" fn mesalink_library_init() -> c_int {
    /* compatibility only */
    1
}

#[no_mangle]
pub extern "C" fn mesalink_add_ssl_algorithms() -> c_int {
    /* compatibility only */
    1
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_load_error_strings() {
    /* compatibility only */
}

#[no_mangle]
pub extern "C" fn mesalink_SSLv3_client_method() -> *mut MESALINK_METHOD {
    let p: *mut MESALINK_METHOD = ptr::null_mut();
    p
}

#[no_mangle]
pub extern "C" fn mesalink_TLSv1_client_method() -> *mut MESALINK_METHOD {
    let p: *mut MESALINK_METHOD = ptr::null_mut();
    p
}

#[no_mangle]
pub extern "C" fn mesalink_TLSv1_1_client_method() -> *mut MESALINK_METHOD {
    let p: *mut MESALINK_METHOD = ptr::null_mut();
    p
}

#[no_mangle]
pub extern "C" fn mesalink_TLSv1_2_client_method() -> *mut MESALINK_METHOD {
    let method = MESALINK_METHOD {
        magic: MAGIC,
        tls_version: rustls::ProtocolVersion::TLSv1_2,
    };
    Box::into_raw(Box::new(method))
}

#[no_mangle]
pub extern "C" fn mesalink_TLSv1_3_client_method() -> *mut MESALINK_METHOD {
    let method = MESALINK_METHOD {
        magic: MAGIC,
        tls_version: rustls::ProtocolVersion::TLSv1_3,
    };
    Box::into_raw(Box::new(method))
}

#[no_mangle]
pub extern "C" fn mesalink_CTX_new(method_ptr: *mut MESALINK_METHOD) -> *mut MESALINK_CTX {
    sanitize_ptr_return_null!(method_ptr);
    let method = unsafe { &*method_ptr };
    let mut client_config = rustls::ClientConfig::new();
    client_config.versions = vec![method.tls_version];
    client_config
        .root_store
        .add_server_trust_anchors(&TLS_SERVER_ROOTS);
    let mut server_config = rustls::ServerConfig::new();
    server_config.versions = vec![method.tls_version];
    let context = MESALINK_CTX {
        magic: MAGIC,
        client_config: Arc::new(client_config),
        server_config: Arc::new(server_config),
    };
    let _ = unsafe { Box::from_raw(method_ptr) };
    Box::into_raw(Box::new(context))
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_new<'a, S: Session, T: Read + Write>(
    ctx_ptr: *mut MESALINK_CTX,
) -> *mut MESALINK_SSL<'a, S, T> {
    sanitize_ptr_return_null!(ctx_ptr);
    let ctx = unsafe { &mut *ctx_ptr };
    let ssl = MESALINK_SSL {
        magic: MAGIC,
        context: ctx,
        hostname: None,
        socket: None,
        session: None,
        stream: None,
    };
    Box::into_raw(Box::new(ssl))
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_set_tlsext_host_name<S: Session, T: Read + Write>(
    ssl_ptr: *mut MESALINK_SSL<S, T>,
    hostname_ptr: *const c_char,
) -> c_int {
    sanitize_ptr_return_fail!(ssl_ptr);
    let ssl = unsafe { &mut *ssl_ptr };
    if hostname_ptr.is_null() {
        mesalink_push_error(ErrorCode::General);
        return SslConstants::SslFailure as c_int;
    }
    let hostname = unsafe { CStr::from_ptr(hostname_ptr) };
    ssl.hostname = Some(hostname);
    SslConstants::SslSuccess as c_int
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_set_fd<S: Session, T: Read + Write>(
    ssl_ptr: *mut MESALINK_SSL<S, T>,
    fd: c_int,
) -> c_int {
    sanitize_ptr_return_fail!(ssl_ptr);
    let ssl = unsafe { &mut *ssl_ptr };
    let socket = unsafe { TcpStream::from_raw_fd(fd) };
    ssl.socket = Some(socket);
    SslConstants::SslSuccess as c_int
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_connect(
    ssl_ptr: *mut MESALINK_SSL<rustls::ClientSession, TcpStream>,
) -> c_int {
    sanitize_ptr_return_fail!(ssl_ptr);
    let ssl = unsafe { &mut *ssl_ptr };
    if let Some(hostname) = ssl.hostname {
        if let Ok(hostname_str) = hostname.to_str() {
            let session = rustls::ClientSession::new(&ssl.context.client_config, hostname_str);
            ssl.session = Some(session);
            let stream = Stream::new(ssl.session.as_mut().unwrap(), ssl.socket.as_mut().unwrap());
            ssl.stream = Some(stream);
            return SslConstants::SslSuccess as c_int;
        }
    }
    mesalink_push_error(ErrorCode::General);
    SslConstants::SslFailure as c_int
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_accept(
    ssl_ptr: *mut MESALINK_SSL<rustls::ServerSession, TcpStream>,
) -> c_int {
    sanitize_ptr_return_fail!(ssl_ptr);
    let ssl = unsafe { &mut *ssl_ptr };

    let session = rustls::ServerSession::new(&ssl.context.server_config);
    ssl.session = Some(session);
    let stream = Stream::new(ssl.session.as_mut().unwrap(), ssl.socket.as_mut().unwrap());
    ssl.stream = Some(stream);
    SslConstants::SslSuccess as c_int
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_read<S: Session, T: Read + Write>(
    ssl_ptr: *mut MESALINK_SSL<S, T>,
    buf_ptr: *mut c_uchar,
    buf_len: c_int,
) -> c_int {
    sanitize_ptr_return_fail!(ssl_ptr);
    let ssl = unsafe { &mut *ssl_ptr };
    let mut buf = unsafe { slice::from_raw_parts_mut(buf_ptr, buf_len as usize) };
    let stream = ssl.stream.as_mut().unwrap();
    match stream.read(&mut buf) {
        Ok(count) => count as c_int,
        Err(_) => {
            mesalink_push_error(ErrorCode::General);
            SslConstants::SslFailure as c_int
        }
    }
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_write<S: Session, T: Read + Write>(
    ssl_ptr: *mut MESALINK_SSL<S, T>,
    buf_ptr: *const c_uchar,
    buf_len: c_int,
) -> c_int {
    sanitize_ptr_return_fail!(ssl_ptr);
    let ssl = unsafe { &mut *ssl_ptr };
    let buf = unsafe { slice::from_raw_parts(buf_ptr, buf_len as usize) };
    let stream = ssl.stream.as_mut().unwrap();
    match stream.write(buf) {
        Ok(count) => count as c_int,
        Err(_) => {
            mesalink_push_error(ErrorCode::General);
            SslConstants::SslFailure as c_int
        }
    }
}

#[no_mangle]
pub extern "C" fn mesalink_CTX_free(ctx_ptr: *mut MESALINK_CTX) {
    let _ = unsafe { Box::from_raw(ctx_ptr) };
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_free<S: Session, T: Read + Write>(ssl_ptr: *mut MESALINK_SSL<S, T>) {
    let _ = unsafe { Box::from_raw(ssl_ptr) };
}