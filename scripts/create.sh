# create k3d cluster
k3d cluster create minimal --servers 1 --agents 0

# merge context
k3d kubeconfig merge minimal -s

# set context
kubectl config use-context k3d-minimal
