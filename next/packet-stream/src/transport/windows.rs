// Copyright 2025 Google LLC
//=============================================================================
// src/socket/windows.rs - Windows Named Pipes with TCP fallback
//=============================================================================

use std::net::SocketAddr;

#[cfg(windows)]
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions};
use tokio::net::{TcpListener, TcpStream};

use crate::error::{PacketStreamError, Result, SocketError};

pub enum WindowsListener {
    #[cfg(windows)]
    NamedPipe {
        server: NamedPipeServer,
        pipe_name: String,
    },
    Tcp {
        listener: TcpListener,
        addr: SocketAddr,
    },
}

pub enum WindowsStream {
    #[cfg(windows)]
    NamedPipe(NamedPipeServer),
    Tcp(TcpStream),
}

impl WindowsListener {
    #[cfg(windows)]
    pub async fn bind_named_pipe(pipe_name: &str) -> Result<Self> {
        let full_name = format!(r"\\.\pipe\{}", pipe_name);

        let server = ServerOptions::new().first_pipe_instance(true).create(&full_name)?;

        Ok(WindowsListener::NamedPipe { server, pipe_name: pipe_name.to_string() })
    }

    #[cfg(not(windows))]
    pub async fn bind_named_pipe(_pipe_name: &str) -> Result<Self> {
        Err(PacketStreamError::Socket(SocketError::UnsupportedPlatform(
            "Named pipes not supported on this platform",
        )));
    }

    pub async fn bind_tcp(addr: SocketAddr) -> Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let addr = listener.local_addr()?;

        Ok(WindowsListener::Tcp { listener, addr })
    }

    pub async fn bind_with_fallback(pipe_name: &str, fallback_addr: SocketAddr) -> Result<Self> {
        #[cfg(windows)]
        {
            match Self::bind_named_pipe(pipe_name).await {
                Ok(listener) => Ok(listener),
                Err(_) => Self::bind_tcp(fallback_addr).await,
            }
        }

        #[cfg(not(windows))]
        {
            Self::bind_tcp(fallback_addr).await
        }
    }

    pub async fn accept(&mut self) -> Result<WindowsStream> {
        match self {
            #[cfg(windows)]
            WindowsListener::NamedPipe { server, pipe_name } => {
                server.connect().await?;

                // Create new server instance for next connection
                let full_name = format!(r"\\.\pipe\{}", pipe_name);
                let new_server = ServerOptions::new().create(&full_name)?;

                // Replace current server with new one and return the connected one
                let connected_server = std::mem::replace(server, new_server);
                Ok(WindowsStream::NamedPipe(connected_server))
            }
            WindowsListener::Tcp { listener, .. } => {
                let (stream, _) = listener.accept().await?;
                Ok(WindowsStream::Tcp(stream))
            }
        }
    }

    pub fn local_addr(&self) -> Result<String> {
        match self {
            #[cfg(windows)]
            WindowsListener::NamedPipe { pipe_name, .. } => Ok(format!(r"\\.\pipe\{}", pipe_name)),
            WindowsListener::Tcp { addr, .. } => Ok(addr.to_string()),
        }
    }
}

#[cfg(windows)]
pub async fn connect_named_pipe(pipe_name: &str) -> Result<NamedPipeServer> {
    let full_name = format!(r"\\.\pipe\{}", pipe_name);
    let client = ClientOptions::new().open(&full_name)?;
    Ok(client)
}

#[cfg(not(windows))]
pub async fn connect_named_pipe(_pipe_name: &str) -> Result<()> {
    Err(PacketStreamError::Socket(SocketError::UnsupportedPlatform(
        "Named pipes not supported on this platform",
    )));
}

pub async fn connect_tcp(addr: SocketAddr) -> Result<TcpStream> {
    let stream = TcpStream::connect(addr).await?;
    Ok(stream)
}
