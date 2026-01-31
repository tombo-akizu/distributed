# apply kustomize config
kubectl apply -k k8s/frontend/
kubectl apply -f k8s/bff/
kubectl apply -f k8s/ingress/middleware.yaml
kubectl apply -f k8s/ingress/ingress.yaml

# host
# kubectl port-forward deploy/frontend 8080:80
