# Modem-Derive

`modem-derive` is a procedural macro crate used by the `modem-rs` library to simplify the parsing of AT commands. It provides a `Command` derive macro that automatically generates parsing logic from a struct definition.

## Architecture

The `Command` macro inspects the fields of a struct and generates an implementation of the `nom` parser for that command. It uses attributes to map struct fields to specific parts of the AT command string.

### `#[command]` Attribute

The `#[command]` attribute is used on a struct to indicate that it represents an AT command. It takes the following arguments:

*   `code`: The AT command code (e.g., `"+CMT"`).
*   `kind`: The type of command, which can be one of `Read`, `Set`, `Exec`, or `Test`.

### Field Attributes

The following attributes can be used on struct fields to control how they are parsed:

*   `#[arg(position)]`: Parses a positional argument from the command string.
*   `#[named_arg(name)]`: Parses a named argument from the command string.
*   `#[flag(value)]`: Parses a flag from the command string.

## Example Usage

Here is an example of how to use the `Command` macro to define an AT command:

```rust
use modem_derive::Command;

#[derive(Command)]
#[command(code = "+CUSD", kind = "Set")]
pub struct SetUssd {
    #[arg(position = 0)]
    pub n: i32,
    #[arg(position = 1)]
    pub str: Option<String>,
    #[arg(position = 2)]
    pub dcs: Option<i32>,
}
```

This will generate a `nom` parser that can parse the `AT+CUSD` command and populate a `SetUssd` struct with the parsed values.
