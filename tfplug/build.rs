fn main() {
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .compile_protos(&["../proto/tfplugin6.9.proto"], &["../proto"])
        .expect("Failed to compile protos");
}