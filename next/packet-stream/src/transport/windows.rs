// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use tokio::net::windows::named_pipe::{
    ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
};

use crate::error::Result;

pub struct WindowsListener {
    server: NamedPipeServer,
    pipe_name: String,
}

impl WindowsListener {
    pub async fn bind_named_pipe(pipe_name: &str) -> Result<Self> {
        let full_name = format!(r"\\.\pipe\{}", pipe_name);

        let server = ServerOptions::new().first_pipe_instance(true).create(&full_name)?;

        Ok(Self { server, pipe_name: pipe_name.to_string() })
    }

    pub async fn accept(&mut self) -> Result<NamedPipeServer> {
        self.server.connect().await?;

        // Create new server instance for next connection
        let full_name = format!(r"\\.\pipe\{}", self.pipe_name);
        let new_server = ServerOptions::new().create(&full_name)?;

        // Replace current server with new one and return the connected one
        let connected_server = std::mem::replace(&mut self.server, new_server);
        Ok(connected_server)
    }

    pub fn local_addr(&self) -> Result<String> {
        Ok(format!(r"\\.\pipe\{}", self.pipe_name))
    }
}

pub async fn connect_named_pipe(pipe_name: &str) -> Result<NamedPipeClient> {
    let full_name = format!(r"\\.\pipe\{}", pipe_name);
    let client = ClientOptions::new().open(&full_name)?;
    Ok(client)
}
