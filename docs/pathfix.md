# Ingress パスルーティング 404問題の調査と修正

## 問題の概要

フロントエンド(`k8s/frontend/html/index.html`)からbffの`/api/echo`エンドポイントにPOSTリクエストを送信すると、404 Not Foundが返される。

## 調査プロセス

### 1. フロントエンドのリクエスト確認

`k8s/frontend/html/index.html`の該当箇所:

```javascript
const url = "http://localhost:8088/api/echo";
// ...
const res = await fetch(url, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(data),
  signal,
});
```

フロントエンドは`/api/echo`にPOSTリクエストを送信している。

### 2. Ingress設定の確認

`k8s/ingress/ingress.yaml`:

```yaml
spec:
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

`/api`プレフィックスのリクエストはbff serviceにルーティングされる。

### 3. bffアプリケーションのルート確認

`apps/bff/src/main.rs`:

```rust
let app = Router::new()
    .route("/", get(root))
    .route("/healthz", get(healthz))
    .route("/echo", post(echo))
```

bffは以下のルートのみ定義:
- `/`
- `/healthz`
- `/echo`

`/api/echo`や`/api/healthz`は**定義されていない**。

### 4. 実際のリクエストテスト

```bash
# Ingress経由でのリクエスト
curl -X POST http://localhost:8088/api/echo -H "Content-Type: application/json" -d '{"test":1}'
# => Status: 404

curl http://localhost:8088/api/healthz
# => Status: 404
```

### 5. bff serviceへの直接リクエストテスト

```bash
kubectl port-forward svc/bff 18082:80 &

# /echo (プレフィックスなし)
curl -X POST http://localhost:18082/echo -H "Content-Type: application/json" -d '{"test":1}'
# => {"test":1}  (成功)

# /api/echo (プレフィックスあり)
curl -X POST http://localhost:18082/api/echo -H "Content-Type: application/json" -d '{"test":1}'
# => Status: 404  (失敗)
```

### 6. Ingress Controllerの確認

```bash
kubectl get pods -A | grep -E "ingress|traefik"
```

結果:
```
kube-system   traefik-5d45fc8cc9-bhb8h   1/1   Running   0   10m
```

**重要な発見**: k3dはデフォルトで**Traefik**をIngress Controllerとして使用している（nginxではない）。

```bash
kubectl get ingressclass
```

結果:
```
NAME      CONTROLLER                      PARAMETERS   AGE
traefik   traefik.io/ingress-controller   <none>       10m
```

## 根本原因

### 問題1: パスがそのまま転送される

Ingressはデフォルトでパスをそのままバックエンドに転送する。

```
クライアント          Ingress              bff service          bff pod
    |                   |                      |                   |
    |--/api/echo------->|                      |                   |
    |                   |--/api/echo---------->|                   |
    |                   |                      |--/api/echo------->|
    |                   |                      |                   |
    |                   |                      |    ルート未定義    |
    |                   |                      |    404 Not Found  |
```

### 問題2: nginx用のアノテーションを使用していた

最初に試した修正:
```yaml
annotations:
  ingress.kubernetes.io/rewrite-target: /$2
```

これはnginx ingress controller用のアノテーションであり、Traefikでは機能しない。

## 修正内容

### 1. Traefik Middlewareの作成

`k8s/ingress/middleware.yaml`を新規作成:

```yaml
apiVersion: traefik.io/v1alpha1
kind: Middleware
metadata:
  name: strip-api-prefix
spec:
  stripPrefix:
    prefixes:
      - /api
```

### 2. IngressにMiddlewareを適用

`k8s/ingress/ingress.yaml`を修正:

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: app-ingress
  annotations:
    traefik.ingress.kubernetes.io/router.middlewares: default-strip-api-prefix@kubernetescrd
spec:
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

アノテーションの形式: `<namespace>-<middleware-name>@kubernetescrd`

### 3. 設定の適用

```bash
kubectl apply -f k8s/ingress/middleware.yaml
kubectl apply -f k8s/ingress/ingress.yaml
```

## 修正後の動作確認

```bash
# /api/echo
curl -X POST http://localhost:8088/api/echo -H "Content-Type: application/json" -d '{"test":1}'
# => {"test":1}  (成功)

# /api/healthz
curl -s -w "\nStatus: %{http_code}\n" http://localhost:8088/api/healthz
# => Status: 200  (成功)

# / (frontend)
curl -s http://localhost:8088/ | head -3
# => <!doctype html>  (成功)
```

## 修正後のリクエストフロー

```
クライアント          Ingress              Middleware           bff pod
    |                   |                      |                   |
    |--/api/echo------->|                      |                   |
    |                   |--/api/echo---------->|                   |
    |                   |                      |--/echo----------->|
    |                   |                      |   (prefix除去)    |
    |                   |                      |                   |
    |                   |                      |    ルート一致     |
    |                   |                      |    200 OK         |
    |<--200-------------|<---------------------|<------------------|
```

## nginx vs Traefik の違い

| 項目 | nginx ingress | Traefik |
|------|--------------|---------|
| パス書き換え方法 | アノテーション + 正規表現 | Middleware CRD |
| アノテーション | `nginx.ingress.kubernetes.io/rewrite-target` | `traefik.ingress.kubernetes.io/router.middlewares` |
| 設定の複雑さ | 単一ファイル | 複数リソース (Middleware + Ingress) |

## 結論

1. k3dはデフォルトでTraefikを使用するため、nginx用のアノテーションは機能しない
2. TraefikではMiddleware CRDを使用してパスの書き換えを行う
3. `stripPrefix`ミドルウェアで`/api`プレフィックスを削除することで問題解決
