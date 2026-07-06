// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

#[derive(Debug, Subcommand, PartialEq)]
pub enum SmsCommand {
    /// Send an SMS message (sms send <sender> <text>)
    Send(SmsSend),
}

#[derive(Debug, Args, PartialEq)]
pub struct SmsSend {
    /// Sender's phone number (digits, optional leading '+', max 15 digits)
    pub sender: String,
    /// SMS text content (options like --id must precede the text)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub text: Vec<String>,
    /// ID of the cellular device
    #[arg(long)]
    pub id: Option<u32>,
}
