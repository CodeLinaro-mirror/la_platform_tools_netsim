//! Backend server library.

#[cxx::bridge(namespace = "netsim")]
mod ffi {

    extern "Rust" {

        #[cxx_name = "StreamPacketHandler"]
        fn stream_packets_handler(packet_stream_client: UniquePtr<PacketStreamClient>);
    }

    unsafe extern "C++" {
        include!("backend/backend_server.h");

        type PacketStreamClient;

        #[rust_name = "read"]
        fn Read(&self) -> UniquePtr<CxxString>;
        #[rust_name = "write"]
        fn Write(&self, response: &CxxString);
    }
}

// A handler for StreamPackets method in c++ Grpc server.
pub fn stream_packets_handler(_client: cxx::UniquePtr<ffi::PacketStreamClient>) {
    // TODO: Stream packets.
}
