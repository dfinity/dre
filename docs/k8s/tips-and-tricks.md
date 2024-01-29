
??? tip "Scale down and up Flux kustomize deployment"

    This allows you to make changes in a cluster without having Flux revert your changes all the time.
    Use it rarely, just during testing, or during troubleshooting.
    Communicate this change with the rest of the team.
    ```
    kubectl -n flux-system scale deployment kustomize-controller --replicas 0 --as root
    ```

    You can scale back up the deployment with:
    ```
    kubectl -n flux-system scale deployment kustomize-controller --replicas 1 --as root
    ```
