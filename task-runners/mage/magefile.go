//go:build mage

// mage — タスクを Go コードで書くビルドツール。`mage <target>`。
// 使い方:
//   mage hello
//   mage test          // mg.Deps(Build) で build を先に実行
//   mage greet Alice   // 引数は関数の型付き引数として受け取る
package main

import (
	"fmt"

	"github.com/magefile/mage/mg"
)

// Hello はあいさつする。
func Hello() {
	fmt.Println("Hello from mage!")
}

// Build はダミービルド。
func Build() {
	fmt.Println("==> building...")
}

// Test は Build に依存する。
func Test() {
	mg.Deps(Build)
	fmt.Println("==> testing...")
}

// Greet は引数 name を受け取る(型付き)。
func Greet(name string) {
	fmt.Printf("Hi, %s!\n", name)
}

// Clean は後片付け。
func Clean() {
	fmt.Println("==> cleaning...")
}
