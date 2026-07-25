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

### 実験場と本体 ── lastshot は自己完結させる

`fastweb/` `pg-bench/` `rust-htmx/` `subsecond-demo/` `connectweb/` `playwright-sample/` `lint-format/`
`task-runners/` は**実験場で、将来まるごと削除する**。したがって:

- **`lastshot/` から実験場を参照してはいけない。** リンクも「詳細は◯◯を見よ」も禁止。
  必要な事実（実測値・制約一覧・公式引用）は**引用して `lastshot/` の中に取り込む**。
- **実験場に新しいドキュメントを足さない。** 残すのは計測を再現するためのコードと手順だけ。
- 逆向き（実験場 → `lastshot/docs/`）の参照はOK。フォルダごと消えるので問題にならない。

判定方法: 以下が何も出なければ自己完結できている。

```sh
grep -rn -E '\.\./(fastweb|connectweb|pg-bench|rust-htmx|subsecond-demo|playwright-sample|lint-format|task-runners)/' \
  lastshot/ --include="*.md" --include="*.toml" --include="*.yml"
```

### 比較用の実装はマージしない

他スタック（Node.js / Laravel など）との比較実装は**ブランチに置いたままにし、main にはドキュメントだけ**を入れる。
ドキュメントからブランチの GitHub URL へリンクする。既存の 2 つのベンチ
（`bench-rust-vs-node` / `feat/lastshot-3stack-compare`）がこの形。

## git 運用

- **マージはユーザー（daijinload）が行う。** Claude はブランチ作成と push まで。
- **指示が無ければ push まで進める。** 「コミットだけ」等の指定が無い場合、Claude は commit → push、
  PR が未作成なら作成まで一気にやる（PR が既にあるブランチなら push すれば自動で乗る）。
  途中で「push しますか？」と訊いて止まらない。
- **force push は必要なときだけ。** 使ってよいのは**履歴を書き換えた直後**（`git rebase` で main を
  取り込んだ / commit を amend した 等）に限る。通常の追加コミットで force を付けない ──
  push が弾かれたら、まず**なぜ弾かれたのか**を確かめること（他所からの push を消しかけている可能性がある）。
- **force push の前に必ずバックアップブランチを切る。徹底する。** 書き換え前の commit を指すブランチを
  ローカルに残してから push する。

  ```sh
  git branch backup/<元ブランチ名>-<YYYYMMDD-HHMM>  # 書き換える前に切る（push 不要・ローカルで十分）
  git push --force-with-lease origin <ブランチ名>
  ```

- **`--force` ではなく `--force-with-lease` を使う。** 手元が知らないうちに進んだ remote を
  問答無用で上書きしない（別 worktree・別マシンからの push を守るため）。

理由: 履歴の書き換えは **git 操作の中で唯一「元に戻せなくなる」種類**なので、戻し先を作ってから実行する。
