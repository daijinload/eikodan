// proto から Connect RPC のコード（メッセージ型・サービス定義）を生成する。
// protoc を PATH から使う（このマシンには protoc 34.1 / buf 1.69 が入っている）。
fn main() {
    connectrpc_build::Config::new()
        .files(&["proto/counter.proto", "proto/report.proto"])
        .includes(&["proto"])
        // 生成型に serde(Serialize/Deserialize) を付ける。これが
        // 「同じ型を HTML テンプレ・埋め込みJSON・Connect API で共有」の土台。
        // （既定でも true だが、connectweb の肝なので明示する）
        .generate_json(true)
        .include_file("_connectrpc.rs")
        .compile()
        .unwrap();
}
