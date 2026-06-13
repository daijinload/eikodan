# GNU Make — Make 代替を比較するための基準。どの環境にも大抵入っている。
# 使い方:
#   make            # 先頭ターゲット(hello)が走る
#   make test       # 依存(build)を解決してから test
#   make greet NAME=Alice

NAME ?= World

.PHONY: hello build test greet clean

hello:                 ## あいさつ
	@echo "Hello from make!"

build:                 ## ダミービルド
	@echo "==> building..."

test: build            ## build に依存
	@echo "==> testing..."

greet:                 ## 引数つき(変数 NAME)
	@echo "Hi, $(NAME)!"

clean:                 ## 後片付け
	@echo "==> cleaning..."
