# bash セットアップ（macOS）

このプロジェクトはタスク実行に `make` ではなく **bash 関数 / bash スクリプト** を使う方針。
ところが macOS 標準の bash は **3.2.57**（2007年でライセンス都合により更新停止）と非常に古く、
連想配列・`${var,,}`・`mapfile` などモダンな機能が使えない。

そこで **Homebrew の bash 5.x を優先して使う** ようにする。

## これは macOS だけの話

**この手順が必要なのは macOS だけ。** Linux/WSL では基本不要。

- macOS が bash 3.2 で止まっているのは技術的理由ではなく**ライセンス都合**（bash 4 以降は GPLv3。
  Apple は GPLv3 を嫌って 2019 年以降の標準シェルを zsh に切り替え、bash は 3.2 のまま放置）。
- 一方、主要な **Linux ディストロは標準で bash 5.x**（Ubuntu/Debian/Arch など）。
  なので Linux では「brew で入れて PATH 優先」みたいなことは要らず、`#!/usr/bin/env bash` でそのまま 5.x。
- つまり **「古い bash 問題」は実質 macOS ローカル限定**。
  スクリプト自体は bash 4+ 前提で普通に書いてよく、macOS の開発機だけこの初期設定をすれば足並みが揃う。

### なぜ Apple は古い bash を残し続けるのか（背景）

「消すか上げるかすればいいのに、なぜ化石を凍結しているのか」には事情がある。Apple は
**「消せない・上げられない」の板挟み**で、GPLv2 最後のバージョンである 3.2 を塩漬けにしている。

- **消せない** → 世の中に `#!/bin/bash` 直書きのスクリプトが大量にある。`/bin/bash` を物理削除すると
  それらが即死する。だから「動くものは互換のために残す」。
- **上げられない** → bash 4 以降は **GPLv3**。Apple は GPLv3（特許条項など）を OS に入れたくないため、
  2019 年以降は標準シェルを zsh に切り替え、bash は 3.2 のまま据え置いた。
- **結果** → bash は「互換のためのレガシー枠」に格下げされ、**飼い殺し**状態。
  非推奨警告すら `BASH_SILENCE_DEPRECATION_WARNING=1` で黙らせる前提になっている。

要するに Apple の本音は **「新規は zsh を使え。bash が要るなら自分で入れろ」**。
このドキュメントの手順（brew で入れて PATH 優先）は、まさにその想定どおりの対処にあたる。

## 結論（最初にこれだけ）

**ゴール：Linux と同じ感覚で「bash 5.x をデフォルトの開発シェル」にする。**
`/bin/bash`（3.2）は置き換えず共存させたまま、brew の bash 5.x を上に被せる。やることは3つ。

1. `brew install bash` で 5.x を入れる
2. それを `/etc/shells` に登録して `chsh` でログインシェルにする
3. `~/.bashrc` に `eval "$(/opt/homebrew/bin/brew shellenv)"` を入れて PATH 優先にする

これでログインシェルも、`bash` と打ったサブシェルも、`env bash` スクリプトも全部 5.x になる。
（下の「[セットアップ手順（ゼロから一式）](#セットアップ手順ゼロから一式)」をそのままなぞればOK）

## 仕組み：3つの経路は別物

「bash を最新にする」と一口に言っても、実は **3つの独立した経路** がある。
混乱の元なので分けて理解する。

| 経路 | 何で決まるか | 例 |
|---|---|---|
| ログインシェル | OS設定（`dscl` の `UserShell`）。**絶対パス**で起動される | ターミナルを開いた瞬間のシェル |
| `bash` と打つ | **PATH の探索順**（先勝ち） | サブシェルを起こす、対話で `bash` |
| `env bash` スクリプト | **PATH の探索順**（先勝ち） | `#!/usr/bin/env bash` なスクリプト |

ログインシェルは PATH ではなく絶対パスで起動するので、PATH をいじっても変わらない（＝壊れない）。
逆に言うと、ログインシェルだけ brew bash にしても、`bash` と打った時は PATH 次第で 3.2 に戻る。
**両方そろえるには「シェル設定」と「PATH 優先」の二段が要る。**

## なぜ放っておくと 3.2 が勝つのか

Homebrew の PATH 登録が `/etc/paths.d/homebrew` 経由だと、macOS の `path_helper` が
`/usr/bin` `/bin` の **後ろ** に `/opt/homebrew/bin` を足す。
その結果、brew bash が入っていても探索順で `/bin/bash`(3.2) に負ける。
だから `.bashrc` で明示的に **前へ** 出す必要がある。

## セットアップ手順（ゼロから一式）

新しい mac でも、これを上から順に実行すれば「bash 5.x をデフォルト開発シェル」にできる。
以下は Apple Silicon（`/opt/homebrew`）前提。Intel Mac は `/opt/homebrew` を `/usr/local` に読み替える。

### 0. Homebrew を入れる（未導入なら）

```bash
# 既に brew があるか確認
command -v brew && brew --version

# 無ければ公式インストーラ
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

インストール直後は brew に PATH が通っていないので、案内に従い（または手動で）一度通す:

```bash
eval "$(/opt/homebrew/bin/brew shellenv)"
```

### 1. brew で最新 bash を入れる

```bash
brew install bash

# 入った場所とバージョンを確認
brew --prefix bash            # → /opt/homebrew/Cellar/bash/5.x.x
/opt/homebrew/bin/bash --version | head -1   # → GNU bash, バージョン 5.x
```

### 2. ログインシェルを brew bash にする

ログインシェルに使えるのは `/etc/shells` に登録されたシェルだけ。まず登録してから `chsh`。

```bash
# /etc/shells に追記（重複登録しないようにチェック）
grep -qxF /opt/homebrew/bin/bash /etc/shells || \
  echo /opt/homebrew/bin/bash | sudo tee -a /etc/shells

# ログインシェルを変更（パスワードを聞かれる）
chsh -s /opt/homebrew/bin/bash

# 確認（OS設定が書き換わっているか）
dscl . -read ~/ UserShell    # → UserShell: /opt/homebrew/bin/bash
```

> ⚠️ `/etc/shells` に **無いパスを `chsh` すると失敗する**。必ず登録を先に。
> 反映はターミナルを開き直してから（既存の窓には効かない）。

### 3. ~/.bashrc で PATH 優先にする（ここが肝）

ログインシェルを変えても、`bash` と打ったサブシェルや `env bash` スクリプトは
PATH 探索で 3.2 に戻りうる（理由は「[なぜ放っておくと 3.2 が勝つのか](#なぜ放っておくと-32-が勝つのか)」）。
`~/.bashrc` の **先頭付近** に次を入れて `/opt/homebrew/bin` を前へ出す:

```bash
# brew のコマンド(bash 5.x 等)をシステム標準(/bin の 3.2)より優先する
eval "$(/opt/homebrew/bin/brew shellenv)"
```

反映は新しいターミナル、または `source ~/.bashrc`。

> 補足：`~/.bash_profile` がある場合、その中で `~/.bashrc` を読んでいるか確認する
> （macOS はログインシェルだと `.bash_profile` を読み、`.bashrc` は自動では読まないため）。
> 無ければ `.bash_profile` の先頭に下記を入れておくと、対話シェルでも確実に `.bashrc` が効く:
>
> ```bash
> [ -f ~/.bashrc ] && . ~/.bashrc
> ```

## 確認

```bash
type bash        # → /opt/homebrew/bin/bash になっていればOK
bash --version   # → GNU bash, バージョン 5.x

which -a bash    # 2つ出るのは正常。/opt/homebrew/bin/bash が先・/bin/bash が後ろなら先勝ちで狙いどおり
```

## メンテ

- `brew upgrade bash` で 5.4 等になっても `/opt/homebrew/bin/bash` を指したままなので、設定はやり直し不要。
- `/bin/bash`（3.2）は SIP 保護で置き換え不可。そもそも置き換える必要はない（消さず共存させる）。

## スクリプト側の保険（任意・配布や CI 向け）

PATH 優先は「自分の環境」にしか効かない。他人の環境や CI でも確実に 5.x を使わせたいなら、
スクリプト先頭にバージョンガードを入れて、古ければ新しい bash で自分を再実行させる。

```bash
#!/usr/bin/env bash
if (( BASH_VERSINFO[0] < 4 )); then
  for b in /opt/homebrew/bin/bash /usr/local/bin/bash; do  # Apple Silicon / Intel
    [[ -x $b ]] && exec "$b" "$0" "$@"
  done
  echo "bash 4+ が必要です (brew install bash)" >&2; exit 1
fi
```

- `BASH_VERSINFO[0]` で今動いている bash のメジャー番号を見る。
- 4未満なら、決め打ちパスにある新しい bash で `exec` 再実行（PATH が古いままでも拾える）。
- 無ければ黙って誤動作せず終了する。

shebang を `#!/opt/homebrew/bin/bash` と **直書きするのは非推奨**（Intel Mac は `/usr/local/bin/bash`、
Linux/CI には存在せず壊れる）。`#!/usr/bin/env bash` のままにしておく。
