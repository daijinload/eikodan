# mask sample

mask — Markdown(maskfile.md)の見出しがそのままタスクになる。`mask <command>`。

使い方:

```
mask hello
mask test          # 本文で $MASK build を呼んで依存を表現
mask greet Alice   # 見出しの (name) が $name に入る
```

## hello

> あいさつ

```sh
echo "Hello from mask!"
```

## build

> ダミービルド

```sh
echo "==> building..."
```

## test

> build に依存($MASK で自分自身を呼ぶ)

```sh
$MASK build
echo "==> testing..."
```

## greet (name)

> 引数つき(見出しの (name) が変数になる)

```sh
echo "Hi, $name!"
```

## clean

> 後片付け

```sh
echo "==> cleaning..."
```
