# apply kustomize config
kubectl apply -k k8s

# host
kubectl port-forward deploy/nginx-html 8080:80
