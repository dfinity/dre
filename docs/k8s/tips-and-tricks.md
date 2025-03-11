
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

??? tip "op-secrets / op-secrets-dre out of sync"

    Usual troubleshooting consists of checking the cluster stores:
    ```bash
    k get clustersecretstore                                                                                     
    NAME                  AGE    STATUS     CAPABILITIES   READY
    op-secret-store       130d   Invalid    ReadOnly       False
    op-secret-store-dre   124d   Invalid    ReadOnly       False   
    ```
    Look into a single one to see what is the issue:
    ```bash
    k get clustersecretstore op-secret-store -o yaml                                                                                         
    apiVersion: external-secrets.io/v1beta1
    kind: ClusterSecretStore
    metadata:
      annotations:
        k8s.dfinity.network/cluster-name: ch1-obs1
      creationTimestamp: "2024-10-31T15:32:23Z"
      generation: 2
      labels:
        k8s.dfinity.network/deployment-stage: bootstrap
        kustomize.toolkit.fluxcd.io/name: system-components
        kustomize.toolkit.fluxcd.io/namespace: flux-deployments
      name: op-secret-store
      resourceVersion: "809835154"
      uid: ce679a85-e255-483b-bd34-286eb44a9cad
    spec:
      provider:
        onepassword:
          auth:
            secretRef:
              connectTokenSecretRef:
                key: token
                name: op-connect-token
                namespace: external-secrets
          connectHost: https://onepassword-connect.op-connect-server.svc.cluster.local:8443
          vaults:
            k8s-secrets: 1
    status:
      capabilities: ReadOnly
      conditions:
      - lastTransitionTime: "2025-01-06T13:52:16Z"
        message: 'unable to validate store: Get "https://onepassword-connect.op-connect-server.svc.cluster.local:8443/v1/vaults?filter=title+eq+%22k8s-secrets%22":
          tls: failed to verify certificate: x509: certificate has expired or is not yet
          valid: current time 2025-03-11T10:50:44Z is after 2025-01-29T14:35:19Z'
        reason: ValidationFailed
        status: "False"
        type: Ready
    ```

    This particular issue is resolved by doing the following steps:

    1. Turn off the kustomize controller
        ```bash
        k scale deployment kustomize-controller --replicas 0 -n flux-system --as root
        ```
    2. Delete the certificate and tls secret for the service
        ```bash
        k delete secret op-connect-tls --as root -n op-connect-server
        k delete certificate op-connect-tls --as root -n op-connect-server
        ```
    3. Scale the kustomize controller up again
        ```bash
        k scale deployment kustomize-controller --replicas 1 -n flux-system --as root
        ```
    4. (Optional) restart the op-connect-server pods:
        ```bash
        k delete po onepassword-connect-6756dc87c7-zwrhg --as root
        ```
    5. Monitor the changes via logs and by looking into the `STATUS` of clustersecretstore from the beginning 
