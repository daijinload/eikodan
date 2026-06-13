# xc sample

xc — README.md などの Markdown 見出しがタスク。タスク定義がそのままドキュメントになる。`xc <task>`。
タスクは `Tasks` 見出し(既定)の配下に 1 段下げて書く。

使い方:

```
xc hello
xc test          # Requires: build で依存を表現
xc greet Alice   # 末尾の引数が $1 に入る
xc -s            # タスク名一覧
```

## Tasks

### hello

あいさつ

```
echo "Hello from xc!"
```

### build

ダミービルド

```
echo "==> building..."
```

### test

build に依存。

Requires: build

```
echo "==> testing..."
```

### greet

引数つき($1、無ければ World)。

```
echo "Hi, ${1:-World}!"
```

### clean

後片付け

```
echo "==> cleaning..."
```
