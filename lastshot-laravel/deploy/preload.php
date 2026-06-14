<?php

// lastshot-laravel の opcache 事前コンパイル(preload)。
// opcache.preload から php-fpm master 起動時に 1 回だけ走り、フレームワークの
// バイトコードをキャッシュする(= 実験メモが要求する「opcache + preload 有効」)。
//
// 注意: preload 中に 1 ファイルでも fatal が出ると php-fpm が起動しなくなる。
// opcache_compile_file は「コンパイルしてキャッシュ」するだけでクラス link は
// しない(未解決依存があっても安全)。各ファイルを try/@ で握りつぶし、preload が
// 失敗してもサーバ起動を妨げないようにする(最悪 opcache 単体の利得は残る)。

if (! function_exists('opcache_compile_file') || ! ini_get('opcache.enable')) {
    return;
}

$base = dirname(__DIR__);

$autoload = $base.'/vendor/autoload.php';
if (is_file($autoload)) {
    require_once $autoload;
}

// Illuminate(フレームワーク本体)+ app(自前コード)を事前コンパイル。
// symfony 等まで舐めると未 link 警告が増える割に得が薄いのでここに絞る。
$dirs = [
    $base.'/vendor/laravel/framework/src/Illuminate',
    $base.'/app',
];

foreach ($dirs as $dir) {
    if (! is_dir($dir)) {
        continue;
    }
    $it = new RecursiveIteratorIterator(
        new RecursiveDirectoryIterator($dir, FilesystemIterator::SKIP_DOTS)
    );
    foreach ($it as $file) {
        if ($file->getExtension() !== 'php') {
            continue;
        }
        try {
            @opcache_compile_file($file->getPathname());
        } catch (\Throwable $e) {
            // preload できないファイルは黙ってスキップ。
        }
    }
}
