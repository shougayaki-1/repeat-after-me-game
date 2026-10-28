# 全探索結果（トークン上限 1,000,000）

2026-10-28に、コミット 6d36d51f2d42b5a280f6a35dd5fe14353b122619 の Rust CLI を実行した結果です。

- 入力長: 1〜8トークン、各長さで全探索（計87,380件）
- 上限: 1,000,000トークン、30ラウンド
- スレッド数: 4、seed: 42（全探索では乱数抽出なし）
- Rust: 1.99.1、macOS aarch64
- 実行: `cargo run --release -- sweep --max-tokens 1000000 --threads 4 --out-dir results`
- 分析: `python3 tools/analyze.py --input-dir results --no-plots`

`summary.csv` は長さ別集計、`rounds.csv` は停止状態とラウンド数の分布、`examples.csv` は代表例です。並列処理の順序により同点の代表例が変わる場合があります。

上限到達は未判定であり、無限に続くことを証明しません。1万トークン上限の実行と比較したところ、各長さの停止状態別の件数は同じでした。
