extern crate grpcio_compiler;

use grpcio_compiler::codegen;

fn main() {
    codegen::protoc_gen_grpc_rust_main();
}
