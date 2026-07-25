# 0001. subsecond ホットパッチ — 不採用

- **判断**: 不採用（2026-06）
- **代わりに採ったもの**: package by feature + 高速リンカ + 約1秒の再起動ループ（[`../cold-start.md`](../cold-start.md)）
- **検証環境**: macOS 26.4.1 / Apple Silicon / Rust 1.91.1 / dioxus-cli 0.7.9

## 検証1: 素の axum に subsecond を足す → **機能しない**

まず lastshot と同じ構成（axum + minijinja）に subsecond クレートだけ足して試した。

- `subsecond::call(...)` で各ハンドラ本体をラップ → `cargo install cargo-hot --locked` → `cargo hot run`
- 起動・ビルド・通常リクエストは成功するが、**ファイル変更時にパッチが送信されない**（dx ログに patch event が一切出ない）
- **原因（ソースで確認）**: `subsecond` クレート自体に CLI ランナーとの**接続/プロトコル実装が無い**
  ── `TcpStream` / `UnixStream` / `env::var` の使用がゼロ。Dioxus / Bevy / Iced が動くのは
  **フレームワーク本体に接続コードが組み込まれているから**であって、クレート単体では成立しない。
- `cargo-hot` 公式 README 冒頭: 「**Currently just an exploration. Very broken! Will eat your laundry!**」
  （[hecrj/cargo-hot](https://github.com/hecrj/cargo-hot)）

→ **素の axum への subsecond 適用は 2026/5 時点で非現実的。** これが不採用の第一の決め手。

## 検証2: Dioxus に乗り換えれば動く（速度自体は出る）

Dioxus 0.7 + `dx serve --hotpatch` で実働デモを組んで実測した。

| 方式                             | サイクル時間（平均） | 内訳                                                    |
| -------------------------------- | -------------------- | ------------------------------------------------------- |
| axum + cargo-watch               | **1405ms**           | 3回計測: 1420 / 1393 / 1403ms                           |
| Dioxus + subsecond（warm）       | **214ms**            | `cargo clean` 後の Session 3 で 263 / 213 / 212 / 218ms |
| 同 cold（dx CLI 初回起動時のみ） | 1255ms               | Session 1 の1回目だけ                                   |

→ 2回目以降は **約6倍速（1.4s → 0.21s）**。300ms を切るので人間の「即時感」のしきい値を跨ぐ。
**速度は十分実用的**だった。

> **「1回目だけ遅い」の正体（仮説が反証された記録）**: 当初「シンボルキャッシュが cold」と仮説を立てたが、
> `cargo clean` で target/ 691MB を完全削除 + 21秒フルリビルドした真の cold 状態で再測定したら **263ms**。
> **仮説は反証された。** dioxus-cli の `packages/cli/src/build/patch.rs` によればキャッシュは
> "the **original module's parsed symbol table**" を**バイナリ単位**で持つもので関数単位ではない。
> だから「別の箇所を編集したら再び遅くなる」は構造上起きない（関数 A→B→C→A の順で編集しても全部 200〜300ms 帯）。
> Session 1 の 1255ms は dx CLI の初回ダウンロード（wasm-bindgen-cli / esbuild）と並走した一回限りの観測。

## なぜ採らなかったか

1. **lastshot のスタック（axum 素組）では動かない**（検証1）。使うには **Dioxus への移行**が必要になる。
   速度のためにフレームワーク全体を入れ替えるのは、HTMX + MiniJinja でビルドゼロ開発を成立させている
   設計を捨てる選択になり、釣り合わない。
2. **制約が開発の実態と噛み合わない。** 特に **struct のレイアウト変更（フィールド追加・型変更）が
   フルビルド**。データモデルを触る頻度は「中〜高」で、モデル拡張のたびに恩恵が消える。
3. **tip crate しかパッチ対象にならない。** lastshot は package by feature で複数クレートに割っており、
   前提が正面から衝突する。`main.rs` + `lib.rs` の二段構成もパッチ不全になる。
4. **experimental 扱い**（Dioxus 0.7 時点の公式表記）。日常の開発ループの土台には置けない。

## 制約の一覧（公式ドキュメントの原文引用つき）

一次ソース: [subsecond docs.rs v0.7.9](https://docs.rs/subsecond/0.7.9/subsecond/) /
[Dioxus 0.7 hot-reload docs](https://dioxuslabs.com/learn/0.7/essentials/ui/hotreload/)

### subsecond クレート本体の限界

| 対象                                                | 挙動                                 | 公式記述                                                                                                                                                     |
| --------------------------------------------------- | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| struct のレイアウト・アライメント変更               | **✗ フルビルド**                     | "Subsecond currently does not support hot-reloading of structs. This is because the generated code assumes a particular layout and alignment of the struct." |
| tip crate 以外（依存crate, workspace member）の編集 | **✗ パッチ非対象**                   | "Rust hot-patching currently only tracks the 'tip' crate in your project. If you edit code in any of your dependencies … DX does not register that change"   |
| `main.rs` + `lib.rs` 二段構成                       | **✗ パッチ機能不全**                 | "Crate setups that have a main.rs importing a lib.rs won't patch sensibly since the crate becomes a library for itself"                                      |
| static initializer 変更                             | **✗ サイレント無視**                 | "Changes to static initializers will not be observed"                                                                                                        |
| 新規 global / static 追加                           | △ パッチ可だがデストラクタ呼ばれない | "You may add new globals at runtime, but their destructors will never be called"                                                                             |
| global / static のリネーム                          | △ 別物として扱われる（状態ロス）     | "Globals are tracked across patches, but renames are observed as introducing a new global"                                                                   |
| `Cargo.toml` 変更（依存追加・feature・version）     | **✗ フルビルド**                     | 直接明記なし。コード生成シードが変わるため当然                                                                                                               |

その他の既知の制約: **iOS 実機は未対応**（codesign 制約。シミュレータは可）、
**thread-local がパッチごとにリセット**される既知 issue。

### Dioxus 0.7 の `--hotpatch` 有効時に依然フルビルドな項目

コンポーネント signature 変更（`#[component]` props の追加/削除）／前回コンパイルに存在しなかった
新しい変数・式／`use` 文・モジュール構造変更／RSX 属性内の複雑な式（関数呼び出しを含むもの）。

### 変更種別ごとのパッチ可否（頻度は qualitative 評価）

公式に明記のない編集について、シンボル単位パッチという仕組みから推定した挙動。
**「頻度」列はデータではなく定性的印象**（プロジェクト・フェーズで偏る）。

| 変更の種類                                     | パッチ可否                                                          | 一般Webアプリでの発生頻度      |
| ---------------------------------------------- | ------------------------------------------------------------------- | ------------------------------ |
| 関数本体・制御フロー・エラーハンドリングの変更 | ✓ パッチ                                                            | **非常に高い**（編集の大半）   |
| 文字列リテラル / 数値定数の変更                | ✓ パッチ                                                            | 高い                           |
| 新しいヘルパー関数の追加                       | ✓ パッチ（新シンボル）                                              | 高い                           |
| `rsx!` 内のテキスト・属性値変更                | ✓ Hotreloading（RSXパス）                                           | 非常に高い                     |
| 新しいハンドラ/ルートの追加                    | △ 関数本体はパッチ可。ルート登録は要再起動の可能性                  | 中                             |
| **struct への新フィールド追加**                | **✗ フルビルド**                                                    | **中〜高（モデル拡張で必発）** |
| struct フィールドの型変更                      | **✗ フルビルド**                                                    | 中                             |
| enum 新バリアント追加                          | docs明記なし。レイアウト依存の可能性                                | 中                             |
| 関数シグネチャに新引数追加                     | docs明記なし。要実機検証                                            | 中                             |
| `Cargo.toml` に依存追加                        | **✗ フルビルド**                                                    | 低（初期に集中）               |
| トレイト定義・新 `impl` ブロック               | docs明記なし。generics 絡みは "cascade of codegen changes" 警告あり | 低〜中                         |

**不確定領域**（実機で確認しない限り断定できないもの）: enum バリアント追加 / 関数シグネチャ変更 /
generic monomorphization の増加 / 新規 `impl` ブロック追加 / マクロ展開結果が変わる変更（特に手続きマクロ）。

### 運用フェーズ別の見立て

- **モデルが安定した運用フェーズ**: 編集の大半が関数本体変更になり 200〜300ms が支配的 → 恩恵大
- **データモデルを毎日いじる初期設計フェーズ**: struct 変更が頻発しフルビルド比率が高い → **恩恵減**

## 併せて見送ったもの: playground crate 方式

subsecond の experimental ステータスを回避する代替として、**最小依存の playground crate を
workspace に足して試作する**案も検討した。これも採らなかった。

- 本体との結合（型を渡す・トレイト境界を共有する）が増えると playground だけで完結しなくなる
- ルーティング・テンプレ・DI 周りは結局本体のビルドが必要で、減らしたかったビルドが減らない
- **package by feature（触った feature だけ再ビルド）で同等の効果が得られた**ので、専用クレートを
  足す理由が消えた

## ひっくり返る条件

- lastshot が Dioxus に移行する、あるいは subsecond が axum など任意の bin で使えるようになる
- struct レイアウト変更がパッチ対象になる
- `--hotpatch` が experimental を外れる

いずれも起きたら再計測する価値がある。現状の約1秒ループが**再起動が要る限りの底**なので
（[`../cold-start.md`](../cold-start.md)）、ここを割るにはプロセスを再起動しない方式が要る。
