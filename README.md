# Hermes -- notifications for Argo Workflows

Hermes aims to provide a streamlined way of sending notifications to various
messaging services from your [Argo Workflows](https://argoproj.github.io/argo-workflows/)
pipelines.

![demo](https://user-images.githubusercontent.com/74944/147889011-6917d13d-dea2-47e5-96bf-5f0d83064816.gif)

Hermes builds upon Argo's yet to be released support for [template executor
plugins](https://github.com/argoproj/argo-workflows/pull/7256). Once you have
installed Hermes in your cluster, Argo will automatically launch an instance of
Hermes for every workflow run and you will be able to interact with it directly
from your workflow.

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

## Supported services

- Slack

## Documentation

Visit the [documentation](https://kjagiello.github.io/hermes) to learn how to
install and use Hermes.
