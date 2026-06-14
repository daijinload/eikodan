# lastshot-bench — 3スタック横並びベンチ

`Desktop/lastshot-3スタック比較-実験メモ.md` の計測部を実装したフォルダ。
**同じ DB・同じ画面・本番ビルド・同じ localhost** で 3 スタックを oha にかけ、
p50/p90/p99/max と RPS を markdown 表で出す。

## 検証したい問い

> 「**DB がボトルネックだから、言語/フレームワークを変えても速度は変わらない**」は本当か？

反対仮説（実証したい方）: **クエリがサブ ms で返る条件では、アプリ層（言語ランタイム ×
アーキテクチャ）の差が支配的になる。** lastshot 実測で DB 往復が <1ms。もう DB は
ボトルネックではない ── なら残りはアプリ層が決める、という主張。

## 対象スタック（各 `../` の別フォルダで独立）

| スタック | 中身 | 本番起動 | ポート |
| --- | --- | --- | --- |
| [lastshot](../lastshot/) | Rust + axum + HTMX + MiniJinja | `./run release`（stable / release / 本番CSS） | 3000+slot |
| [lastshot-next](../lastshot-next/) | Node.js + Next.js（App Router・RSC） | `./run prod`（next build → next start） | 3100+slot |
| [lastshot-laravel](../lastshot-laravel/) | PHP + Laravel（Blade） | `./run prod`（nginx + php-fpm + opcache/preload + artisan optimize） | 3200+slot |

`slot` は worktree 名の末尾数字（`eikodan`→0 / `dan3`→3）。3 スタックとも `./run` が
同じ機構でポート/DB を決めるので、worktree を分ければ並列起動してもぶつからない。

## 揃えた条件（フェアにするための統制）

- **全く同じ DB を共有**: 同じネイティブ PostgreSQL・同じ `counter` テーブル（1 行 id=1）・
  同じ unix ソケット（`/tmp`）・同じ database 名（worktree スロットに追従。例 `lastshot_dan3`）。
  3 スタックとも `select value ... where id=1` / `update ... +1 ... returning value` の同一 SQL。
  スキーマは lastshot の Flyway が所有し、他 2 つは**マイグレーションせず既存テーブルを読むだけ**。
- **全く同じ画面**: 数字 ＋「+1」ボタンだけ。CSS は 3 つとも手書きで見た目を合わせ、
  CSS フレームワークのビルド差を交絡にしない（lastshot だけ release は daisyUI 生成CSS）。
- **本番相当ビルドで測る**（dev mode で測らない）。各スタックの素の serving モデルで動かす:
  - lastshot: 1 つの axum/tokio プロセスが全コアを使う
  - next: 1 つの Node プロセス（`next start`）
  - laravel: nginx + php-fpm（static ワーカ = コア数×2）。opcache + preload + `artisan optimize`
- 同じマシン・localhost・同じ瞬間に計測。

## 使い方

各スタックを別ターミナルで本番起動してから（`tests-http` と同じ「サーバは別起動」方式）:

```sh
# それぞれ別ターミナルで
(cd ../lastshot        && ./run release)
(cd ../lastshot-next   && ./run prod)
(cd ../lastshot-laravel && ./run prod)
```

そのうえで:

```sh
./run all          # GET / と POST increment を 3スタック計測 → 表（既定 c=50, 10s）
CONN=1  ./run all  # 無負荷レイテンシ（p50/p99 のテール）
CONN=50 ./run all  # 飽和（最大RPS）
./run weight       # 1画面あたりの HTML サイズ / サブリソース参照数（fan-out の目安）
./run urls         # 各スタックの想定 URL（起動先確認）
./run counter      # 共有 DB のカウンタ現在値
./run reset        # カウンタを 0 に戻す（計測後の片付け）
```

`DURATION`（既定 `10s`）・`CONN`（既定 `50`）・`WARMUP`（既定 `2s`）は env で上書き。
各行は「ウォームアップ（捨て）→ 本計測」で、V8 JIT / opcache を温めてから測る。

## 実測結果（参考値・2026-06-14）

環境: Apple Silicon mac（darwin arm64・15 論理コア）/ ネイティブ PostgreSQL 17（unix ソケット）/
lastshot = Rust release(stable) axum ・ next = Next.js 15.5（`next start`・1 Node プロセス）・
laravel = Laravel 13.15 on nginx 1.31 + php-fpm 8.5（static 30 ワーカ）opcache+preload。
**localhost で oha(クライアント) と 3 サーバが同じ CPU を取り合う**点に注意（下記「落とし穴」）。

### 無負荷レイテンシ（c=1）── p50/p99 のテールを見る

| stack | route | p50(ms) | p90(ms) | p99(ms) | max(ms) | RPS |
|---|---|---|---|---|---|---|
| lastshot | GET | 0.053 | 0.059 | 0.073 | 0.308 | 18516 |
| next | GET | 0.762 | 0.853 | 1.339 | 4.561 | 1264 |
| laravel | GET | 4.366 | 5.154 | 6.091 | 7.321 | 230 |
| lastshot | POST | 0.054 | 0.061 | 0.078 | 0.405 | 18064 |
| next | POST | 0.157 | 0.175 | 0.262 | 10.640 | 5973 |
| laravel | POST | 4.236 | 4.952 | 5.765 | 8.797 | 234 |

### 飽和（c=50）── 最大 RPS を見る

| stack | route | p50(ms) | p90(ms) | p99(ms) | max(ms) | RPS |
|---|---|---|---|---|---|---|
| lastshot | GET | 0.900 | 0.988 | 1.093 | 6.905 | 55010 |
| next | GET | 29.764 | 31.840 | 37.825 | 90.258 | 1657 |
| laravel | GET | 26.638 | 28.164 | 29.911 | 36.676 | 1875 |
| lastshot | POST | 1.139 | 1.302 | 1.596 | 19.600 | 41524 |
| next | POST | 6.368 | 7.966 | 10.214 | 54.980 | 7317 |
| laravel | POST | 26.936 | 29.013 | 32.266 | 41.491 | 1847 |

### 1画面あたりの重さ（fan-out の目安）

| stack | html 本体 | サブリソース参照数 |
|---|---|---|
| lastshot | 1533 bytes | ~1（htmx + CSS。下記注） |
| next | 5228 bytes | 8（hydration 用 JS chunk 群） |
| laravel | 2142 bytes | 0（CSS/JS インライン） |

## 追加検証: Next.js を PM2 cluster で多重化したら差は縮むか（c=20・2026-06-15）

> 仮説: 「**lastshot は 1 プロセス全コア（15論理コア）、laravel は php-fpm 30 ワーカで多重化、
> next は 1 Node プロセスだから不利**。c=20 で next が 0.68 秒まで詰まったのは React SSR が重い
> ことに加えて 1 プロセスが CPU バウンドな描画を直列にさばいているから。next を PM2 で 15
> プロセスに多重化したら、上限 RPS は数倍に上がり、1/100 という比率は 1/7 程度まで縮むはず。」

`lastshot-next` に `./run prod-cluster`（PM2 cluster mode × 15 ワーカ・同一ポートを Node の
cluster モジュールで共有）を足し、**同じ機械・同じ瞬間** で single と cluster を撃ち比べた。
3 スタックとも localhost で oha と一緒に CPU を取り合う。

### 重い一覧画面 GET /report?rows=1000（c=20・10s）── 仮説の主戦場

| stack（serving）| p50(ms) | p90(ms) | p99(ms) | max(ms) | RPS | vs lastshot |
|---|---|---|---|---|---|---|
| lastshot（axum 全コア1プロセス） | 6.993 | 7.151 | 7.994 | 9.916 | 2854 | 1.00 |
| next **single**（1 Node プロセス） | 645.697 | 673.001 | 863.601 | 1069.665 | **31** | **1/92** |
| next **cluster x15**（PM2 cluster） | 82.013 | 142.296 | 202.463 | 248.522 | **208** | **1/14** |
| laravel（nginx + php-fpm 30 ワーカ） | 20.262 | 28.447 | 37.036 | 42.334 | 949 | 1/3.0 |

→ **仮説は方向としては正しかった**: next の RPS は 31 → 208（約 **6.7 倍**）、p50 は 646ms → 82ms
（約 **1/8**）、lastshot との比は **1/92 → 1/14**。0.68 秒で詰まっていたのは確かに「1 プロセスが
CPU バウンドな描画を直列にさばいていた」せい。ただし **1/7 までは縮まなかった**（実測 1/14）。
理由は localhost で oha + 15 next ワーカ + Rust の axum 全コア + Postgres + nginx + php-fpm が
**同じ CPU を取り合う**ことと、React SSR の絶対コストが Rust の MiniJinja より重い分。

### 軽い画面 GET /（c=20・10s）── もとから飽和していない経路

| stack | single p50(ms) | single RPS | cluster p50(ms) | cluster RPS | RPS 倍率 |
|---|---|---|---|---|---|
| lastshot | 0.360 | 54847 | 0.360 | 54917 | 1.00x |
| next | 12.753 | 1547 | 4.459 | 4159 | **2.7x** |
| laravel | 10.674 | 1869 | 11.506 | 1733 | 0.93x |

### 薄い JSON POST /api/increment（c=20・10s）── React を挟まない経路

| stack | single p50(ms) | single RPS | cluster p50(ms) | cluster RPS | RPS 倍率 |
|---|---|---|---|---|---|
| lastshot | 0.446 | 40942 | 0.443 | 41106 | 1.00x |
| next | 2.872 | 7255 | 1.217 | 14957 | **2.1x** |
| laravel | 10.920 | 1829 | 11.786 | 1670 | 0.91x |

### 読み筋

- **重い SSR ほど多重化の効きが大きい**: /report = 6.7x、/ = 2.7x、POST = 2.1x。
  cluster の利得は「**1 プロセス時にどれだけ CPU 上限で詰まっていたか**」の関数。
  /report の single は p50 646ms、CPU 1 コアに張り付いて完全に直列化していた典型例。
  軽い / や薄い POST はもともと飽和していないので、台数を増やしても CPU 余地ぶんしか効かない。
- **逆転は起きていない**: 重い画面ですら lastshot は依然 14 倍速い。多重化は「1 言語/ランタイムの
  上限を引き上げる」だけで、ランタイム差そのものはなくならない。Laravel も /report で 949 RPS と、
  cluster 化した Next の 208 RPS より速い（php-fpm 30 ワーカが効いている / Blade の SSR が React より軽い）。
- **同一マシン同時計測の弊害**: cluster 計測中の Laravel は RPS が single 計測時より少し下がっている
  （1536 → 949 等）。これは next が 15 ワーカで CPU を奪った副作用で、絶対値ではなく **同一行内の
  single vs cluster の差** で読むのが正解。
- **「PM2 cluster は事実上必須」という結論にはしない**: lastshot リポジトリのこのベンチが
  「**素のアーキテクチャでの比較**」を意図しているため、本表は `prod-cluster` を**追加実験**として
  並置し、本来の比較対象（`./run prod` = 1 Node プロセス）は据え置く。実運用で Next.js を出すなら
  まず PM2 cluster や Vercel/Cloud Run のような前段多重化を入れる、という運用上の結論はここに残す。

### 再現コマンド

```sh
# それぞれ別ターミナルで
(cd ../lastshot         && ./run release)
(cd ../lastshot-next    && ./run prod-cluster)   # ← PM2 cluster × 15 ワーカ
(cd ../lastshot-laravel && ./run prod)

# 計測（CONN=20 で同じ瞬間に撃つ）
CONN=20 DURATION=10s ./run all
CONN=20 DURATION=10s ROWS=1000 ./run report
```

`INSTANCES`（既定 15）と `PG_POOL_MAX`（既定 4。15×4=60 < PostgreSQL の `max_connections=100`）は
`lastshot-next/run` の env で上書き可。`single` 版は `./run prod` に戻して同じ計測を回す。

## 結論（この実測が言っていること）

- **DB がサブ ms で返る条件では、差はアプリ層でつく。** 同じ DB・同じ SQL なのに GET の
  RPS は lastshot ≫ next ≒ laravel で **30 倍以上**開く。「DB がボトルネックだから言語は
  関係ない」は、この条件では成り立たない ── 反対仮説が支持された。
- **p50 でも既に差は大きい**（メモの「p50 は意外と差が小さいかも」より差は大きく出た）。
  c=1 GET の p50 は lastshot 0.05ms / next 0.76ms / laravel 4.4ms。Laravel は opcache+preload
  ありでも 1 リクエストごとのフレームワーク bootstrap が効いている。
- **アーキテクチャ差が言語差に上乗せされる。** Next.js は GET（RSC + SSR）が重い一方、
  POST `/api/increment`（React を挟まない素の JSON ルート）は GET の数倍速い ── 同じ Node でも
  「画面 SSR」か「薄い API」かで桁が変わる。lastshot は GET/POST がほぼ同じ（HTMX フラグメント）。
- **fan-out**: Next.js の画面は hydration 用に複数 chunk を引く（参照 8）。HTMX/Blade は
  1 リクエストで完結（参照 ~1 / 0）。「1 画面 = 何リクエストか」もアーキテクチャの差。

## 落とし穴（数値の読み方）

- **localhost の CPU 取り合い**: oha と 3 サーバが同じマシンの CPU を共有する。特に c=50 の
  飽和域は、相手サーバや他プロセスの負荷で数値が上下する（この実測でも、マシンが他作業で
  忙しい時の c=50 は lastshot RPS が c=1 より低く出た）。**順序関係は頑健**だが、絶対値は
  「同条件・同時刻での横並び」として読む。厳密には専用クライアントマシンから測るべき。
- **POST は同一行（id=1）を更新**するので高並列だと行ロックで直列化する。これは 3 スタック
  共通（同じ DB・同じ行）なので比較としては相殺されるが、「POST の最大 RPS」は DB 行ロックの
  天井も混じる。アプリ層差が一番きれいに見えるのは **GET と c=1 レイテンシ**。
- **サブリソース参照数は素の HTML を grep した目安**。複数行に分かれた `<script>` は数え落とす
  （lastshot の htmx が未カウントで ~1 と出る）。正確な「実リクエスト数」はブラウザ HAR で測る
  （[`../lastshot/browser/`](../lastshot/browser/) の Playwright が実 swap/表示を見る土台）。
- **serving モデルは各スタックの素のまま**（axum=全コア1プロセス / next=1 Node プロセス /
  laravel=php-fpm 30 ワーカ）。これも「言語 × アーキテクチャ」の一部として込みで見る。

## なぜ oha か

p50/p90/p99/max とRPS をまとめて JSON で出せて（`--output-format json`）、`-z`（時間）/
`-c`（並列）/`-m`（メソッド）が素直。`brew install oha` 一発。結果は `jq` で抜いて表に整形している。
