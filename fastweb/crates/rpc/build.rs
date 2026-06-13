// proto から Connect RPC のコード（メッセージ型・サービス定義）を生成する。
// protoc を PATH から使う（このマシンには protoc 34.1 / buf 1.69 が入っている）。
fn main() {
    connectrpc_build::Config::new()
        .files(&["proto/greet.proto"])
        .includes(&["proto"])
        .include_file("_connectrpc.rs")
        .compile()
        .unwrap();
}
