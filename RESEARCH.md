# Repeat After Me Game — 大規模検証

既存のWebシミュレーター (`index.html`) と同じ規則を使って、初期文字列を全探索またはランダムサンプリングするRust製CLIです。分析・グラフ作成にPythonを使用します。

## 必要環境

- Rust（stable、`cargo`）
- Python 3.9以降（分析用）。グラフには任意で `matplotlib` が必要です。
- Rust側は外部ライブラリ不要です。

## 実行

リポジトリのルートで実行してください。

```bash
# まず正しさを検証
cargo test
python3 tools/validate_reference.py

# ホワイトボードの例を、命令ごとに追跡
cargo run --release -- trace 'RAM nRAM KON'

# 小規模の全探索（デフォルト: 1〜8トークン）
cargo run --release -- sweep

# 12トークンまで全探索（長時間になる可能性あり）
cargo run --release -- sweep --min-length 1 --max-length 12 --threads 8 --out-dir results

# 15トークンを一様ランダムに10万回抽出して検証（復元抽出）
cargo run --release -- sweep --min-length 15 --max-length 15 --sample-size 100000 --seed 42 --threads 8 --out-dir results-15

# CSVを集計して、任意でPNGグラフ生成
python3 tools/analyze.py --input-dir results --output-dir reports
```

`--threads` は利用できるCPUコア数に合わせて変更してください。`--help` で他のオプションが確認できます。

## アルゴリズム

トークンは `RAM`, `KON`, `nRAM`, `nKON` の4種類です。

- `RAM` は自身より右側の全トークンをそのまま出力します。
- `nRAM` は自身より右側の全トークンを `RAM↔KON`, `nRAM↔nKON` で同時変換して出力します。
- 元の文字列内に存在する **すべての命令** が同じ元文字列を参照して独立に動き、出力を左から順に連結したものが次のラウンドになります。
- `KON` と `nKON` は命令ではありません。

例：`RAM nRAM KON` → `nRAM KON RAM` → `RAM KON` → `KON`。

Rust内部ではトークンを2bit値（0,1,2,3）で表し、`nRAM` の反転をXOR `1` で処理しています。次の文字列長は生成前に計算します。

## 停止条件と解釈

- **done**：命令がなくなった。空文字列も正常終了。
- **cycle**：過去と完全一致する文字列が再出現した。**真の循環が証明される**。
- **token_limit**：次の計算結果が指定トークン数を超えるので、生成前に停止（デフォルト10,000）。
- **round_limit**：指定ラウンド数に到達（デフォルト30）。

後ろの2つは **無限に続くと確定したわけではありません**。最終結果ではなく「上限で停止した時点の状態」です。制限に達するまでのラウンド数を記録します。

全探索の組み合わせ数は長さ `n` ごとに `4^n` です。`n=12` は16,777,216通りです。誤操作による極端な全探索を避けるため、各長さで2,000万通りを超える場合は自動的にエラーにします（`--max-cases` で明示的に変更可能）。`--sample-size` を指定した場合は、同じ入力が複数回抽出される可能性があります。

## 出力ファイル

- `results/summary.csv`：入力長ごとの調査件数、各停止条件の件数と割合、平均ラウンド数、最大長など。
- `results/rounds.csv`：入力長 × 停止状態 × 実行ラウンド数の分布。
- `results/examples.csv`：各停止状態の代表入力と、文字列長が最大の例。

数百万件を1行ずつ保存すると容量が大きくなるため、通常は統計量と代表例のみ保存します。ファイル名や計算の上限は必要に応じて調整できます。

## 注意

Web版は初期文字列が空である場合、入力エラーとします。Rust CLI の `trace` も空入力を受け付けません。全探索は長さ1から開始します。

並列処理の順序に依存して同点の代表例が変わることがあります。集計件数は変わりません。検証コードは `src/main.rs` 内のRust単体テストと `tools/validate_reference.py` のPython参照実装を用意しています。GitHub ActionsでもRustのコンパイルとテストを行います。

## GitHub上で実行する場合

GitHubリポジトリの **Actions → Run large-scale sweep → Run workflow** で、最大トークン長（8 / 10 / 12）、全探索かサンプリングか、上限を指定して実行できます。結果はワークフローの `research-results` アーティファクト（CSV・分析テキスト）から取得できます。60分のタイムアウト付きで、GitHub Actionsの使用枠を消費します。Webサイト上ではなくGitHubのサーバーで計算されます。
