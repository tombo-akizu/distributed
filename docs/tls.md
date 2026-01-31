# TLS終端の設定

## 目的

フロントエンド（ブラウザ）とTraefik間をHTTPS通信にし、Traefikとbff間はHTTP通信とする構成を実現する。

## 構成図

```
ブラウザ                    Traefik                 Backend Pods
   │                          │                        │
   │──HTTPS (port 8443)──────▶│                        │
   │    TLS暗号化              │──HTTP (port 80)───────▶│
   │    証明書検証             │    平文(クラスタ内部)   │
   │                          │                        │
   │◀─────────────────────────│◀───────────────────────│
```

**TLS終端 (TLS Termination)**: TraefikでTLSを復号し、バックエンドへは平文で転送する方式。

## 実装手順

### 1. 自己署名証明書の生成

```bash
mkdir -p k8s/ingress/certs

openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout k8s/ingress/certs/tls.key \
  -out k8s/ingress/certs/tls.crt \
  -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
```

| オプション | 説明 |
|-----------|------|
| `-x509` | 自己署名証明書を生成 |
| `-nodes` | 秘密鍵を暗号化しない |
| `-days 365` | 有効期間1年 |
| `-newkey rsa:2048` | 2048ビットRSA鍵を新規生成 |
| `-subj "/CN=localhost"` | Common Name設定 |
| `-addext "subjectAltName=..."` | SAN (Subject Alternative Name) 設定 |

### 2. Kubernetes Secretの作成

```bash
kubectl create secret tls tls-secret \
  --cert=k8s/ingress/certs/tls.crt \
  --key=k8s/ingress/certs/tls.key \
  --dry-run=client -o yaml > k8s/ingress/tls-secret.yaml
```

生成されたYAML (`k8s/ingress/tls-secret.yaml`):

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: tls-secret
type: kubernetes.io/tls
data:
  tls.crt: <base64エンコードされた証明書>
  tls.key: <base64エンコードされた秘密鍵>
```

### 3. Ingressの更新

`k8s/ingress/ingress.yaml`にTLS設定を追加:

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: app-ingress
  annotations:
    traefik.ingress.kubernetes.io/router.middlewares: default-strip-api-prefix@kubernetescrd
spec:
  tls:                          # TLS設定を追加
    - hosts:
        - localhost
      secretName: tls-secret    # 作成したSecretを参照
  rules:
    - host: localhost
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: frontend
                port:
                  number: 80
          - path: /api
            pathType: Prefix
            backend:
              service:
                name: bff
                port:
                  number: 80
```

### 4. k3dクラスタのポートマッピング更新

`scripts/create.sh`を更新してHTTPSポートのみを公開:

```bash
k3d cluster create mycluster \
  --servers 1 \
  --agents 0 \
  -p "8443:443@loadbalancer"    # HTTPSのみ
```

| ポート | 用途 |
|--------|------|
| 8443:443 | HTTPS (Traefik websecure entrypoint) |

HTTPポートは公開しない（セキュリティ強化のため）。

### 5. フロントエンドのURL更新

`k8s/frontend/html/index.html`のAPIエンドポイントを相対URLに変更:

```javascript
// 変更前
const url = "http://localhost:8088/api/echo";

// 変更後（HTTP/HTTPS両対応）
const url = "/api/echo";
```

相対URLを使用することで、ページがHTTPSで配信された場合、APIリクエストも自動的にHTTPSになる。

### 6. run.shの更新

`scripts/run.sh`にtls-secret.yamlの適用を追加:

```bash
kubectl apply -k k8s/frontend/
kubectl apply -f k8s/bff/
kubectl apply -f k8s/ingress/tls-secret.yaml  # 追加
kubectl apply -f k8s/ingress/middleware.yaml
kubectl apply -f k8s/ingress/ingress.yaml
```

## クラスタの再作成と適用

```bash
# 既存クラスタを削除
bash scripts/delete.sh

# 新しいポートマッピングでクラスタ作成
bash scripts/create.sh

# リソースを適用
bash scripts/run.sh
```

## 動作確認

### Podの起動確認

```bash
kubectl wait --for=condition=ready pod -l app=bff --timeout=60s
kubectl wait --for=condition=ready pod -l app=frontend --timeout=60s
```

### HTTPS接続テスト

```bash
# -k オプションで自己署名証明書の検証をスキップ
curl -sk https://localhost:8443/api/healthz
# => (200 OK)

curl -sk -X POST https://localhost:8443/api/echo \
  -H "Content-Type: application/json" \
  -d '{"message":"hello via HTTPS"}'
# => {"message":"hello via HTTPS"}

curl -sk https://localhost:8443/ | head -3
# => <!doctype html>
# => <html lang="ja">
# => <head>
```

### HTTP接続テスト（無効化確認）

```bash
curl -s --connect-timeout 2 http://localhost:8088/
# => 接続不可（期待通り）
```

HTTPポートは公開していないため、接続できないことを確認。

## ファイル構成

```
k8s/ingress/
├── certs/
│   ├── tls.crt          # 証明書
│   └── tls.key          # 秘密鍵
├── ingress.yaml         # TLS設定付きIngress
├── middleware.yaml      # パス書き換えMiddleware
└── tls-secret.yaml      # TLS Secret

scripts/
├── create.sh            # HTTPSポート追加
└── run.sh               # tls-secret適用追加
```

## Traefik Entrypoints

Traefikはデフォルトで以下のEntrypointsを持つ:

| Entrypoint | ポート | 用途 |
|------------|--------|------|
| web | 8000 (内部) → 80 (公開) | HTTP |
| websecure | 8443 (内部) → 443 (公開) | HTTPS |

IngressにTLS設定を追加すると、Traefikは自動的に`websecure` entrypointでリクエストを受け付ける。

## 注意事項

### 自己署名証明書について

- 開発環境専用。本番環境ではLet's Encrypt等の正規の証明書を使用する
- ブラウザで警告が表示される（「この接続ではプライバシーが保護されません」）
- curlでは`-k`オプションで証明書検証をスキップ

### 証明書のGit管理

秘密鍵(`tls.key`)はGitにコミットしないこと。`.gitignore`に追加推奨:

```gitignore
k8s/ingress/certs/
```

本番環境では:
- cert-manager + Let's Encrypt
- 外部シークレット管理（Vault, AWS Secrets Manager等）
- sealed-secrets

### Mixed Content

HTTPSページからHTTP APIを呼び出すとブラウザがブロックする。相対URLを使用することでこの問題を回避。
