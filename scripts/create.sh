# create k3d cluster
k3d cluster create mycluster \
  --servers 1 \
  --agents 0 \
  -p "8088:80@loadbalancer"

docker build -t bff:1.0.0 -f apps/bff/Dockerfile apps/bff
k3d image import bff:1.0.0 -c mycluster
