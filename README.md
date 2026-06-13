# eikodan

理想のWebシステムを模索するための曳光弾（えいこうだん）プロジェクトです
モノレポでサンプルを作っていきます！！

## 要件

* システムが高速に動作すること
* 開発が高速にできること
* 出来るだけシンプルに開発できること
  * Reactの複雑性を排除してHTMXを使う
  * Rust側でMiniJinjaを使うことによりHTMX部分はコンパイル無しで開発できる
* 自動テストがあること

## agent-browser のインストール

ブラウザ操作・スクショ・QA を AI エージェントから行うための CLI（[vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser)）を使っています。

```sh
# CLI 本体をグローバルにインストール
npm i -g agent-browser
agent-browser install
```

リポジトリ直下の `.agents/` と `skills-lock.json` が skill の定義・ロックファイルです（コミット済みのものをそのまま使えます）。
