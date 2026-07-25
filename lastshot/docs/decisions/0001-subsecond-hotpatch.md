# 0001. subsecond ホットパッチ — 不採用

- **判断**: 不採用（2026-06）
- **検証場所**: [`../../../subsecond-demo/`](../../../subsecond-demo/)（Dioxus 0.7 + `dx serve --hotpatch` の実働デモ）
- **代わりに採ったもの**: package by feature + 高速リンカ + 約1秒の再起動ループ（[`../cold-start.md`](../cold-start.md)）

## 何を試したか

実行中の WASM バイナリに**シンボル単位でパッチを当てる** subsecond で、Rust コード変更の反映を
どこまで縮められるかを実測した。axum + cargo-watch のフルプロセス再起動（約1.4秒）に対し、
**subsecond は2回目以降 約200〜300ms ＝ 約6倍速**という数字自体は出た。

## なぜ採らなかったか

1. **lastshot のスタック（axum 素組）では動かない。** subsecond は Dioxus の `dx` ツールチェーンに
   組み込まれた機能で、使うには **Dioxus への移行**が必要になる。速度のためにフレームワーク全体を
   入れ替えるのは、HTMX + MiniJinja でビルドゼロ開発を成立させている設計と釣り合わない。
2. **制約が開発の実態と噛み合わない。** 特に **struct のレイアウト変更（フィールド追加・型変更）が
   フルビルド**。データモデルを触る頻度は「中〜高」で、モデル拡張のたびに恩恵が消える。
3. **tip crate しかパッチ対象にならない。** lastshot は package by feature で複数クレートに割っており、
   前提が正面から衝突する。`main.rs` + `lib.rs` の二段構成もパッチ不全になる。
4. **experimental 扱い**（Dioxus 0.7 時点の公式表記）。日常の開発ループの土台には置けない。

制約の完全な一覧（公式ドキュメントの原文引用つき）と、変更種別ごとのパッチ可否表は
[`subsecond-demo/README.md`](../../../subsecond-demo/README.md) に残してある。

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
