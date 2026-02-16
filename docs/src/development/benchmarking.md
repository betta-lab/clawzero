# Benchmarking

clawzero には Claude Code・OpenClaw との性能比較を Docker 上で再現可能に実行するベンチマーク環境が含まれています。

## クイックスタート

```bash
# 全ツール・全シナリオを実行
docker compose -f bench/docker-compose.yml run bench

# clawzero の起動時間のみ計測
docker compose -f bench/docker-compose.yml run bench --tools clawzero --scenarios startup

# イテレーション数を指定
docker compose -f bench/docker-compose.yml run bench --iterations 10
```

## 前提条件

- Docker / Docker Compose
- `ANTHROPIC_API_KEY` 環境変数（API を使うシナリオに必要）
- `OPENAI_API_KEY` 環境変数（OpenClaw 用、任意）

## 計測メトリクス

| メトリクス | 計測方法 |
|---|---|
| 起動時間 (cold start) | `hyperfine` で `--help` 実行の壁時計時間 |
| TTFT (Time to First Token) | カスタムラッパーで stdout の最初の 1 バイト到着までの時間 |
| E2E 完了時間 | `hyperfine` でプロンプト実行の壁時計時間 |
| メモリ使用量 (peak RSS) | `/usr/bin/time -v` の Maximum resident set size |
| トークンスループット | 出力文字数 / E2E 時間 |

## シナリオ

| シナリオ | 内容 | API コール |
|---|---|---|
| `startup` | `--help` の実行時間 | なし |
| `simple` | `"What is 1+1?"` への応答 | あり |
| `tool_use` | ファイル読み取り＋行数カウント | あり |

## ファイル構成

```
bench/
├── Dockerfile              # マルチステージビルド
├── docker-compose.yml      # 環境変数とボリュームマウント
├── run.sh                  # メインベンチマークランナー
├── adapters/
│   ├── clawzero.sh         # clawzero 呼び出しアダプタ
│   ├── claude-code.sh      # Claude Code 呼び出しアダプタ
│   └── openclaw.sh         # OpenClaw 呼び出しアダプタ
├── measure_ttft.sh         # TTFT 計測ヘルパー
├── fixtures/
│   └── bench_input.txt     # tool_use シナリオ用テストファイル
└── results/                # 結果出力 (.gitignore)
```

## run.sh のオプション

```
--tools <t1,t2,...>       計測対象ツール (default: clawzero,claude-code,openclaw)
--scenarios <s1,s2,...>   実行シナリオ (default: startup,simple,tool_use)
--iterations <N>          反復回数 (default: $BENCH_ITERATIONS or 5)
--results-dir <path>      結果出力先 (default: bench/results)
```

## 環境変数

| 変数名 | 説明 | デフォルト |
|---|---|---|
| `ANTHROPIC_API_KEY` | Anthropic API キー | (必須) |
| `OPENAI_API_KEY` | OpenAI API キー | (任意) |
| `BENCH_ITERATIONS` | 反復回数 | `5` |
| `BENCH_MODEL` | clawzero で使用するモデル | `anthropic/claude-sonnet-4-5-20250929` |

## 結果

結果は `bench/results/<timestamp>/` に保存されます:

- `results.json` — 全メトリクスの JSON
- `<tool>_<scenario>_hyperfine.json` — hyperfine の raw データ
- `<tool>_<scenario>_time.txt` — `/usr/bin/time` の出力
- `<tool>_<scenario>_ttft.csv` — TTFT の CSV データ

実行終了時にサマリーテーブルがコンソールに出力されます。

## アダプタの追加

新しいツールを追加するには `bench/adapters/<name>.sh` を作成し、以下の関数を定義します:

```bash
TOOL_NAME="my-tool"

cmd_startup() {
    my-tool --help
}

cmd_simple() {
    my-tool "What is 1+1?"
}

cmd_tool_use() {
    my-tool "Read /tmp/bench_input.txt and count the lines"
}
```

`--tools my-tool` で指定すると自動的に読み込まれます。
