# connectweb

**スキーマ（.proto）を「単一の真実」に置き、1つの生成型から HTML・埋め込みJSON・Connect API
の3経路を同時に駆動する** Rust スタックの曳光弾。[fastweb](../fastweb/) のビルド高速化と
package-by-feature をそのまま土台にし、その上に「スキーマファーストの共存」を載せたもの。

## fastweb との違い（何が新しいか）

| | fastweb | connectweb |
|---|---|---|
| HTMLに渡すデータ | 手書きの context（`title => "..."`） | **proto 生成型インスタンス** |
| スキーマ | connect-rpc は任意の脇役 | **.proto が中心。型の単一定義** |
| 「この画面のデータ」 | 見えない | **同じインスタンスを HTML 末尾に JSON 埋め込み** |
| API と画面の一致 | 別々 | **同じ `get_user`・同じ型から両方が生まれる** |

土台（MiniJinja 実行時テンプレ / HTMX / daisyUI / lld / opt非対称 / sccache / 葉クレート）は
fastweb と同一。ビルド速度の実測根拠は [`fastweb/BENCHMARK.md`](../fastweb/BENCHMARK.md)。

## 設計の核（aaa.txt の議論の結論）

```
            .proto（唯一の真実 / ビュー専用メッセージ UserPageView）
               │ buffa+connectrpc で生成（serde 標準装備 = json feature）
               ▼
        サービス層 get_user(id) -> UserPageView   ← ロジックの本体。ここだけ書けばいい
         │                          │
  HTML経路(feature-user)      API経路(rpc)
   render_view で             同じ get_user を呼ぶだけ
   ├ minijinja に描画         └ POST /user.v1.UserService/GetUser
   └ 同じ instance を末尾に        （HTMLと同一ポート同居・自分への通信ゼロ）
     <!-- view-data --> コメントで埋め込み
```

ポイント:
- **ビュー専用スキーマ**: 画面に出す約束のフィールドだけを `UserPageView` に定義する。
  「雑に全データ入りの型を作って一部だけ使う」はやらない ── これがそのまま情報境界になる。
- **データ生成は1回・出口は2つ**: `render_view` が同じインスタンスを描画と埋め込みに使うので、
  「画面の値」と「埋め込みJSONの値」がズレようがない（別APIを叩くとズレ得る問題を回避）。
- **3経路は排他ではない**: HTMLだけ / APIだけ / 両方、どれでも成立する。スキーマを真ん中に
  置いてあるので、後から経路を足すのは薄い殻を1つ書くだけ。

## セットアップ（初回だけ）
```sh
brew install sccache    # 必須。.cargo/config.toml が rustc-wrapper に指定しているため。
# protoc が必要（schema/build.rs の codegen が使う）。未導入なら: brew install protobuf
```

## 動かす
```sh
cargo run -p app
# → http://127.0.0.1:3000 を開く（id=1 のユーザー画面）。/users/7 など id を変えられる。
#   view-source すると末尾の <!-- view-data --> コメントに「その画面が使った正確なデータ」。
#   「再読み込み」ボタンはアクティビティ一覧を HTMX で部分更新（再ビルド無し。断片は先頭に
#   同じ view-data コメントが付くので DevTools の Network→Response で中身を確認できる）。
```

### Connect API（HTMLと同一ポートで同居 / 同じ get_user を共有）
```sh
curl -X POST http://127.0.0.1:3000/user.v1.UserService/GetUser \
  -H "Content-Type: application/json" -d '{"id":7}'
# => {"id":7,"name":"ユーザー7","email":"user7@example.com","role":"member",
#     "recentActivities":[{"action":"ログイン","at":"2026-06-13T09:00:00Z"}, ...]}
```
画面末尾の埋め込みJSONとキー構造（**camelCase**）・値が一致することを確認できる。

## 開発ループ（推奨）
```sh
bacon            # 保存で cargo check が即返る（型エラーを最速で拾う）
bacon run        # サーバーを起動し、Rust変更で自動再起動
bacon serve      # ↑をソケット引き継ぎ再起動に（接続が切れない。要 `cargo install systemfd`）
```

## HTTPテスト（別プロジェクト・起動は外部）
```sh
cargo run -p app &              # 起動
cd tests-http && cargo nextest run
```

## 構成
```
crates/
  app/            bin。ルーター組み立て・起動・ライブリロードだけの薄い層
  webcore/        共有コア（AppState + MiniJinjaローダ）。render（素） と render_view（埋め込みJSON付き）
  schema/         .proto + build.rs codegen = 単一の真実。HTML/JSON/API が共有する生成型
  feature-user/   1機能 = get_user(サービス層) + HTMLルート + templates/（依存は webcore と schema だけ）
  rpc/            Connect API の薄い殻。get_user を呼んで同じ型を返す
tests-http/       起動済みサーバーをHTTPで叩くブラックボックステスト（ワークスペース外）
```

## 注意点（スキーマファースト由来）
- **テンプレも埋め込みJSONも camelCase**: buffa の serde は proto3 JSON 準拠なので、proto の
  `recent_activities` はテンプレートで `view.recentActivities`。snake_case では参照できない。
- **codegen が走るのは proto を変えたときだけ**: 生成は `schema` クレートに隔離。feature や
  rpc の Rust を編集しても再生成は起きない（= fastweb のビルド速度がそのまま保たれる）。
- 高速化フラグの位置は fastweb と同じ（リンカ/threads/sccache=`.cargo/config.toml`、
  nightly=`rust-toolchain.toml`、opt非対称/Cranelift=`Cargo.toml`）。
