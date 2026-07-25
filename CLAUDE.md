# CLAUDE.md（eikodan リポジトリ共通の方針）

このファイルは eikodan リポジトリで作業するときに自動で読み込まれる、Claude 向けの指示です。
**ここに書かれた方針は eikodan リポジトリ内でのみ有効**で、他のプロジェクトには影響しません。

## ドキュメント方針

プロジェクトの知識（各サブプロジェクトの目的・セットアップ手順・設定の意図など）は、
**Claude の個人メモリではなくリポジトリ側のドキュメントに残す**こと。

- **調査結果・実測値・設計判断 → [`lastshot/docs/`](./lastshot/docs/) に集約する。**
  lastshot が本体なので、知見はすべてここに集める。新しく足したら
  [`lastshot/docs/README.md`](./lastshot/docs/README.md) の索引に**必ず1行足す**
  （足せないなら新規ファイルを作らず既存ドキュメントの節にする）。ファイル名は `lower-kebab-case.md`。
- **採らなかった判断も残す → [`lastshot/docs/decisions/`](./lastshot/docs/decisions/)。**
  理由と実測値がないと同じ検討を二度やるので、消さない。
- 使い方・セットアップ手順 → 各サブプロジェクトの `README.md`。
- Claude への横断的な指示・運用ルール → このルート `CLAUDE.md`、または各サブプロジェクトの `CLAUDE.md` に書く。
- **Claude の個人メモリ（`~/.claude/.../memory/`）には eikodan のプロジェクト知識を保存しない。**
  記録が必要なら上記のリポジトリ内ドキュメントに追記する。

理由: git でバージョン管理・チーム共有でき、リポジトリを見れば全体像が分かる状態を保つため。

### 実験場と本体

`fastweb/` `pg-bench/` `rust-htmx/` `subsecond-demo/` `connectweb/` `playwright-sample/` `lint-format/`
`task-runners/` は**実験場（アーカイブ）**。結論は `lastshot/` に取り込み済みで、ドキュメントは
`lastshot/docs/` に移してある。**実験場に新しいドキュメントを足さない**こと（再現用のコードと手順だけ残す）。

### 比較用の実装はマージしない

他スタック（Node.js / Laravel など）との比較実装は**ブランチに置いたままにし、main にはドキュメントだけ**を入れる。
ドキュメントからブランチの GitHub URL へリンクする。既存の 2 つのベンチ
（`bench-rust-vs-node` / `feat/lastshot-3stack-compare`）がこの形。
