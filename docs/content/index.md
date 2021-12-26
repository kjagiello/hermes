# Hermes – notifications for Argo Workflows

Hermes aims to provide a streamlined way of sending notifications to various
messaging services from your [Argo Workflows](https://argoproj.github.io/argo-workflows/)
pipelines.

<figure markdown>
  ![demo](assets/demo.gif)
  <figcaption>An example of a Slack notification sent using Hermes</figcaption>
</figure>

## Features

- **Easy to use** – Hermes is a [template executor
  plugin](https://github.com/argoproj/argo-workflows/pull/7256). Once
  installed, Argo Workflows will automatically provide a Hermes instance for you to
  interact with from your workflow.
- **Template system** – keep a centralized set of reusable notification
  templates and use them freely in your workflows.
- **In-place updates** – avoid clutter in your channels by updating existing
  messages and keep the history of changes in a thread under the notification
  message instead.
- **Multiple recipient support** – do you need to send notifications to different
  channels or services from a single workflow? No problem.

## Quickstart

{% include "content/compatibility.md" %}

Keen to take Hermes for a spin? Go ahead and visit the [quickstart guide](quickstart.md).

## Usage example

In case you need some more convincing before you give Hermes a chance, take a
look at an example workflow that sends the notifications shown in the demo
above.

```yaml
{% raw %}
apiVersion: argoproj.io/v1alpha1
kind: Workflow
metadata:
  generateName: notifications-test-
spec:
  entrypoint: main
  templates:
    - name: main
      steps:
        - - name: setup-notifications
            template: hermes-setup

        - - name: pre-notification
            template: hermes-notify
            arguments:
              parameters:
                - name: message
                  value: "Deployment started :hourglass_flowing_sand:"

        - - name: hello
            template: hello

        - - name: post-notification
            template: hermes-notify
            arguments:
              parameters:
                - name: message
                  value: "Deployment succeeded :white_check_mark:"

    - name: hermes-setup
      plugin:
        hermes:
          setup:
            alias: default
            service: slack
            config:
              token: slack-token
              icon_emoji: ":rocket:"

    - name: hermes-notify
      inputs:
        parameters:
          - name: message
      plugin:
        hermes:
          notify:
            target: default
            template: hermes-template-slack-default
            config:
              channel: sandbox
            context:
              message: "{{inputs.parameters.message}}"
              app: hermes
              env: prod
              revision_sha: "deadbeef"
              revision_url: "http://github.com/..."
              log_url: "http://github.com/..."

    - name: hello
      container:
        image: docker/whalesay
        command: [cowsay]
        args: ["hello world"]
{% endraw %}
```

If this managed to catch your interest, learn how to setup Hermes using the [quickstart guide](quickstart.md).
