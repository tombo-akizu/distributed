# BFF Pod CrashLoopBackOff 問題の調査と修正

## 問題の概要

k3dクラスタ上でbff podがCrashLoopBackOff状態となり、正常に起動しない問題が発生。

## 調査プロセス

### 1. Pod状態の確認

```bash
kubectl get pods -o wide
```

結果:
```
NAME                       READY   STATUS             RESTARTS       AGE
bff-ddb6dbb97-cg5qd        0/1     CrashLoopBackOff   11 (96s ago)   33m
frontend-8889dbd55-9rnvt   1/1     Running            0              33m
```

### 2. Podの詳細確認

```bash
kubectl describe pod -l app=bff
```

重要な発見:
```
Last State:     Terminated
  Reason:       Completed
  Exit Code:    0
  Started:      Sat, 31 Jan 2026 11:06:55 +0900
  Finished:     Sat, 31 Jan 2026 11:06:55 +0900
```

- Exit Code: 0 (正常終了)
- 開始と終了がほぼ同時刻

これはコンテナが起動直後に正常終了していることを示す。エラーではなく、プロセスが即座に終了している。

### 3. Dockerイメージの動作確認

```bash
docker run --rm -d --name bff-test -p 18081:8081 bff:1.0.0
docker logs bff-test
```

結果: コンテナが出力なしで即座に終了。

### 4. Dockerfileの分析

`apps/bff/Dockerfile`の内容:

```dockerfile
# ---- build stage ----
FROM rust:1.85 as builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs  # ダミーmain.rs作成
RUN cargo build --release                              # ダミーをビルド
RUN rm -rf src                                         # srcを削除

COPY . .                                               # 本物のソースをコピー
RUN cargo build --release                              # ← 問題箇所
```

### 5. ビルドログの確認

```bash
docker build --no-cache -t bff:test -f apps/bff/Dockerfile apps/bff
```

ビルドログ:
```
#12 Compiling bff v0.1.0 (/app)        # ダミーをコンパイル
#12 Finished `release` profile [optimized] target(s) in 9.11s

#13 [builder 6/8] RUN rm -rf src
#14 [builder 7/8] COPY . .
#15 [builder 8/8] RUN cargo build --release
#15 Finished `release` profile [optimized] target(s) in 0.03s  # 再コンパイルなし!
```

## 根本原因

Dockerfileのマルチステージビルドにおいて、依存関係キャッシュの最適化手法に問題があった。

1. ダミーの`fn main(){}`を作成してビルド（依存関係のキャッシュ用）
2. srcを削除して本物のソースをコピー
3. `cargo build --release`を実行

しかし、cargoは`target/`ディレクトリ内のキャッシュを参照し、ソースファイルの変更を検知しない。結果として、ダミーの空バイナリがそのまま使用された。

空の`fn main(){}`は即座に終了するため、Kubernetesはコンテナの再起動を繰り返し、CrashLoopBackOff状態となった。

## 修正内容

`apps/bff/Dockerfile`を修正:

```diff
 COPY . .
-RUN cargo build --release
+RUN touch src/main.rs && cargo build --release
```

`touch src/main.rs`によりファイルのタイムスタンプを更新し、cargoに変更を検知させて再コンパイルを強制する。

## 修正後の確認

### ビルドログ

```
#15 [builder 8/8] RUN touch src/main.rs && cargo build --release
#15 Compiling bff v0.1.0 (/app)    # 本物のソースがコンパイルされた
#15 Finished `release` profile [optimized] target(s) in 0.90s
```

### Dockerイメージのテスト

```bash
docker run --rm -d --name bff-fixed -p 18081:8081 bff:1.0.0
docker logs bff-fixed
# => listening on http://0.0.0.0:8081

curl http://localhost:18081/healthz
# => (200 OK)
```

### クラスタへのデプロイ

```bash
k3d image import bff:1.0.0 -c mycluster
kubectl rollout restart deployment/bff
kubectl get pods
# => bff-79d667fcc7-wvj2v   1/1   Running   0   15s
```

## 結論

問題はDockerfileのcargoキャッシュ戦略にあり、`touch`コマンドの追加で解決した。scriptsディレクトリ以下のスクリプトは変更不要。
