A service, in context of Hermes, provides an integration with a messaging
service, i.e. Slack, Teams, etc. Note that every service have different
requirements as for configuration (authentication, templates, etc) and also
provide different set of capabilities (Slack supports in-place updates of
notifications, while IRC would not). For this reason, this guide will only show
the service-agnostic parts of interacting with Hermes.

## Service configuration

Let's start with a minimal workflow in which we call two templates that will be
defined later on in this guide.

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
        - - name: setup-service
            template: hermes-setup

        - - name: send-notification
            template: hermes-notify
            arguments:
              parameters:
                - name: message
                  value: "Hello world!"

    - name: hermes-setup
      # To be defined...

    - name: hermes-notify
      # To be defined...
{% endraw %}
```

### Setting up a service

A service is setup by issuing a `setup` call to Hermes. It expects following
parameters:

- `alias` – an alias for the service that we will use to send
  notifications. This allows us to have multiple instances of the same service
  (imagine a scenario when you would like to send notifications to multiple
  Slack workspaces from the same workflow)
- `service` - the name of the service that we want to setup, e.g. `slack`
- `config` - service specific configuration, e.g. authenthication token, custom
  avatar, etc

```yaml title="hermes-setup"
- name: hermes-setup
  plugin:
    hermes:
      setup:
        alias: default
        service: some-service
        config:
          # Service specific config
```

### Adding a template

In order to send a notification, we have to setup a template that we will use
to render the notification. A template at its core is just a
[ConfigMap](https://kubernetes.io/docs/concepts/configuration/configmap/) that
has following shape:

```yaml title="hermes-template"
{% raw %}
apiVersion: v1
kind: ConfigMap
metadata:
  name: hermes-template
data:
  subtemplate1: |
    {"message": "{{message}}"}
  # subtemplate2: ...
{% endraw %}
```

The `{{message}}` you are seeing in the template above is a [Handlebar
expression](https://handlebarsjs.com/guide/#simple-expressions). It will be
populated with the context provided by you when sending a notification.
Internally, Hermes uses
[handlebar-rust](https://github.com/sunng87/handlebars-rust) as the template
engine.

Also, as you probably have noticed, a template consists of sub-templates.
Sub-templates allow services to have multiple ways of presenting the same
notification, i.e. in case of Slack a message might be sent to the channel
using the `primary` sub-template and updates to the message will be posted in
the message thread using the `secondary` sub-template. Every service defines
its own set of required sub-templates.

### Sending a notification

Now that we have both the service and template setup, it is time to see how
sending notifications works. Notifications are sent using the `notify` call to
Hermes. It expects following parameters:

- `target` – the alias of the service that we want to use to send a notification
- `config` - service specific configuration, e.g. name of a slack channel
- `template` - the name of the template to use for the notification
- `context` - the context to render the template with

```yaml title="hermes-notify"
{% raw %}
- name: hermes-notify
  inputs:
    parameters:
      - name: message
  plugin:
    hermes:
      notify:
        target: default
        config:
          # Service specific config
        template: hermes-template
        context:
          # Template context
          message: "{{inputs.parameters.message}}"
{% endraw %}
```

### Complete workflow

Putting all the puzzle pieces together we end up with the following workflow.

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

        - - name: send-notification
            template: hermes-notify
            arguments:
              parameters:
                - name: message
                  value: "Hello world!"

    - name: hermes-setup
      plugin:
        hermes:
          setup:
            alias: default
            service: some-service
            config:
              # Service specific config

    - name: hermes-notify
      inputs:
        parameters:
          - name: message
      plugin:
        hermes:
          notify:
            target: default
            template: hermes-template
            config:
              # Service specific config
            context:
              # Template context
              message: "{{inputs.parameters.message}}"
{% endraw %}
```

The workflow above is of course not entirely complete as the service-specific
parts are still missing, but replacing the service-specific parts with actual
config should result in you seeing a "Hello world!" message in the messagin
service of your choosing.

## What's next?

Now that you are familiar with the core concepts of Hermes, you will be able
to use this information to setup an actual service. Choose one of the supported
services below and start sending notifications:

- [Slack](slack/index.md)
