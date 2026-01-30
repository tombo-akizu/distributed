# environment
- install kubectl
  - i must know the processor architecture
  - i run `uname -m` and get x86_64.
  - install based on [kubernetes install guide](https://kubernetes.io/ja/docs/tasks/tools/install-kubectl-linux/)
    - curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
    - sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl
    - kubectl version --client
      ```
      Client Version: v1.35.0
      Kustomize Version: v5.7.1
      ```
- install docker engine
  - install based on [docker install guide](https://docs.docker.com/engine/install/ubuntu/)
    - install docker-engine because docker-desctop is likely unsupported (https://qiita.com/tf63/items/c21549ba44224722f301)
    - docker --version
      ```
      Docker version 29.2.0, build 0b9d198
      ```
    - add the user in docker group
      - sudo usermod -aG docker $USER
- install k3d
  - install based on [k3d install guide](https://k3d.io/stable/#releases)
    - curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash
  - k3d --version
    ```
    k3d version v5.8.3
    k3s version v1.31.5-k3s1 (default)
    ```

## minimum cluster
