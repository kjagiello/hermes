# Quick Start

{% include "content/compatibility.md" %}

## Install Hermes

Install Hermes by creating following the ConfigMap in your cluster:

=== "plugin.yaml"

    ```yaml
    --8<-- "plugin.yaml"
    ```

=== "kubectl"

    ```yaml
    kubectl apply -f \
      {{ raw_github_url("plugin.yaml") }}
    ```

!!! hint

    Keep in mind that template executor plugins run as containers within a
    single pod, thus port collisions can occur. If your encounter this issue,
    you might have to adjust the port in the plugin manifest of Hermes.

## Service account

Authentication tokens for the different services are passed to Hermes as
secrets, which in turn requires that Hermes is able to fetch them using the
Kubernetes API. Argo Workflows, by default, uses a service account with limited
permissions, so in order to successfully run Hermes you will have to create a
custom Role for your workflow that grants the `get` permission to the secrets
needed by Hermes.

See an example below:

```yaml
---
# Role
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: workflow-role
rules:
  # Pod get/watch is used to identify the container IDs of the current pod.
  # Pod patch is used to annotate the step's outputs back to controller (e.g. artifact location).
  - apiGroups:
      - ""
    verbs:
      - get
      - watch
      - patch
    resources:
      - pods
  # Logs get/watch are used to get the pods logs for script outputs, and for log archival
  - apiGroups:
      - ""
    verbs:
      - get
      - watch
    resources:
      - pods/log
  # Access to secrets
  - apiGroups:
      - ""
    verbs:
      - get
    resources:
      - secrets
    resourceNames:
      # List your secrets here
      - ...

---
# RoleBinding
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: workflow-permissions
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: workflow-role
subject:
  kind: ServiceAccount
  name: workflow-sa

---
# ServiceAccount
apiVersion: v1
kind: ServiceAccount
metadata:
  name: workflow-sa
```

## What's next?

Now that Hermes is installed it is time to take a look on how to send some
notifications. In order to do that, let's get yourself familiarized with
[services](services/index.md).
