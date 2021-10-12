# Introduction

Today, we will examine how we can leverage Service Binding Operator (SBO) to
make connecting services to applications easier within a kubernetes cluster.

# An example

As an example, let's say I have two kubernetes services, `producer` and
`consumer`, that talk to a RabbitMQ instance using AMQP.  `producer`
periodically produces data that `consumer` reads and acts on.  For the sake of
this demonstration, that action is printing whatever it receives to `stdout`.

In normal circumstances (read: not running on kubernetes), we would need to tell
both `producer` and `consumer` how to connect to a rabbitmq instance.  This
means distributing to these services the following information:

- Hostname/Port
- Authentication credentials (such as username & password)

In summary, for our example, we want the following:

- An operator-managed RabbitMQ cluster running on kubernetes (we will use
  https://github.com/rabbitmq/cluster-operator for this demonstration)
- Our `producer` and `consumer` also running on kubernetes

# Connecting the hard way

As a comparison, let's connect our services to our rabbitmq cluster without
using SBO.

TODO: What is "best practice" for connecting a service to a rabbitmq cluster
without SBO?

# Connecting made easier

Instead of doing this, we can instead leverage SBO to get these services to talk
to RabbitMQ.

TODO: detail [jobs.yaml](/jobs.yaml) and [service-binding.yaml](/service-binding.yaml)

## An even easier way

Set the label `service.binding/provisioned-service=true` on the custom resource
(instead of the annotations we would usually set) and everything should work.

TODO: (insert details of it working here)

## The easiest way

The RabbitMQ operator is a part of the service binding registry, so these
annotations/labels that we set earlier don't even need to be set for this to
work.
