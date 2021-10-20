# Introduction

Today, we will examine how we can leverage the Service Binding Operator (SBO)
to make connecting services to applications easier within a kubernetes cluster.

# Background - What is the Service Binding Operator?

In the realm of Kubernetes, exposing secrets to applications to allow them to
connect to services is an inconsistent process across the ecosystem.  Many
service providers have their own bespoke method of binding an application to
their service.

The Service Binding Operator is intended to remedy this issue by managing the
binding process for us.  When you execute a binding request, the operator looks
at information stored within the custom resource and its corresponding custom
resource definition.  This information informs the operator of the information
it needs to expose to the application and will project it into its runtime
container.  It does so either by environment variables or by files mounted
within the container.

To learn more about some of the other features SBO supports and its
integrations with other products, you can read our release announcement
[here](https://docs.google.com/document/d/1VgTMKlc9B1_32hGT1AnZhzEjomGKwlTYc4nZ7QudkpU/edit#),
which covers those details.  In this post, we will be looking at an example of
binding in action using the Service Binding Operator.

# An example

Let's say I have two kubernetes services, `producer` and `consumer`, that talk
to a RabbitMQ instance using AMQP.  `producer` periodically produces data that
`consumer` reads and acts on.  For the sake of this demonstration, that action
is printing whatever it receives to `stdout`.

We'll return to the concept of binding once we have everything setup.  For now,
let's get our RabbitMQ cluster setup on a local kubernetes cluster (I prototype
with `minikube`, but the instructions would be the same if you were to run this
on an OpenShift cluster).

First, we can setup the RabbitMQ operator:
```bash
kubectl apply -f https://github.com/rabbitmq/cluster-operator/releases/latest/download/cluster-operator.yml
```

Next, we'll need to install Operator Lifecycle Manager (OLM), a prerequisite
for the Service Binding Operator:
```bash
curl -sL https://github.com/operator-framework/operator-lifecycle-manager/releases/download/v0.19.1/install.sh | bash -s v0.19.1
```

NOTE: yes, doing `curl ... | bash` isn't the best in terms of security.  If this
is a concern for you, you can instead save the installation script to a location
in your filesystem and execute the script from there after inspecting its
contents.

We'll also need the Service Binding Operator:
```bash
kubectl apply -f https://operatorhub.io/install/service-binding-operator.yaml
```

Next, we want to have our `producer` and `consumer` running on our kubernetes
cluster.  For convenience, I've authored two containers that provide this
functionality; their sources can be found
[here](https://github.com/sadlerap/sbo-rabbitmq-sample).

SBO can operate against deployments, and deployments make the most sense for
running our applications.  You can deploy them with the following:
```bash
kubectl create deployment producer --image=quay.io/ansadler/rabbitmq-producer:latest
kubectl create deployment consumer --image=quay.io/ansadler/rabbitmq-consumer:latest
```

We'll also want a rabbitmq cluster to run them against:
```yaml
apiVersion: rabbitmq.com/v1beta1
kind: RabbitmqCluster
metadata:
  name: rabbitmq
spec:
  service:
    type: ClusterIP
```

If you wish, you can deploy all of these (that is, `producer`, `consumer`, and
a rabbitmq cluster) with the following:
```bash
kubectl apply -f https://raw.githubusercontent.com/sadlerap/sbo-rabbitmq-sample/master/jobs.yaml
```

NOTE: Why don't we need to add [service binding
annotations](https://redhat-developer.github.io/service-binding-operator/userguide/exposing-binding-data/adding-annotation.html)
here?  RabbitMQ already exposes the information necessary to be recognized as a
provisioned service, so SBO already knows how to connect to it.

Now, if you inspect our container logs for `consumer`, you'll see something
similar to this:
```
$ kubectl logs consumer-deployment-f877cffb6-p9sks
Error:
   0: $RABBITMQCLUSTER_HOST not defined

Location:
   src/consumer.rs:16

Backtrace omitted.
Run with RUST_BACKTRACE=1 environment variable to display it.
Run with RUST_BACKTRACE=full to include source snippets.
```

`producer` throws a similar error.  This is because we haven't bound our
RabbitMQ cluster to either `producer` and `consumer`, so neither of them know
where to find it.  Let's fix that.

## Binding things together

If we were not using SBO, we would need to tell both `producer` and `consumer`
how to connect to a rabbitmq instance.  This means distributing at minimum the
following information to these services:

- The hostname of the RabbitMQ instance
- The port that the RabbitMQ instance is listening on
- Authentication credentials (such as username and password)

This would require us to expose our secrets to our applications, either by
having these applications directly fetch that information from kubernetes' API
or by projecting that information into our applications ourselves.  Both of
these methods are rather intrusive into our applications, and it stands to
reason that this process could be automated.

To be able to bind our applications and services together, Service Binding
Operator introduces a new custom resource titled `ServiceBinding`, which
represents the binding between an application and a service.  In this
particular example, the bindings for our `producer` and `consumer` applications
would look like this:

```yaml
---
apiVersion: binding.operators.coreos.com/v1alpha1
kind: ServiceBinding
metadata:
  name: servicebinding-consumer
spec:
  bindAsFiles: false
  services:
  - group: rabbitmq.com
    version: v1beta1
    kind: RabbitmqCluster
    name: rabbitmq
  application:
    name: consumer-deployment
    version: v1
    group: apps
    resource: deployments
---
apiVersion: binding.operators.coreos.com/v1alpha1
kind: ServiceBinding
metadata:
  name: servicebinding-producer
spec:
  bindAsFiles: false
  services:
  - group: rabbitmq.com
    version: v1beta1
    kind: RabbitmqCluster
    name: rabbitmq
  application:
    name: producer-deployment
    version: v1
    group: apps
    resource: deployments
---
```

NOTE: If we were running this against a different operator (one not supported
in our [service
registry](https://github.com/redhat-developer/service-binding-operator#known-bindable-operators)),
we would have to give SBO permission to read from this service via RBAC rules.
You can read more about how to do that
[here](https://redhat-developer.github.io/service-binding-operator/userguide/exposing-binding-data/rbac-requirements.html)

Now, if we inspect the logs of our `consumer` deployment, we'll see that we've
been receiving messages from our `producer`.  You should see something similar
to the following:

```
$ kubectl logs consumer-deployment-6f48dbfb7d-5dsgd
connecting to: amqp://default_user_7Jba_ZP7NKD-UjJK8AQ:HIhVZ4a_6Xm60Z7bmbEDADDpwr2e_tch@rabbitmq.default.svc:5672
Waiting for messages, press Ctrl-C to exit.
(  0) Received [hello, world!]
(  1) Received [hello, world!]
(  2) Received [hello, world!]
(  3) Received [hello, world!]
(  4) Received [hello, world!]
(  5) Received [hello, world!]
(  6) Received [hello, world!]
(  7) Received [hello, world!]
(  8) Received [hello, world!]
(  9) Received [hello, world!]
( 10) Received [hello, world!]
( 11) Received [hello, world!]
( 12) Received [hello, world!]
```

`producer` says something similar:

```
kubectl logs producer-deployment-6d8d55949d-8qd9c
connecting to: amqp://default_user_7Jba_ZP7NKD-UjJK8AQ:HIhVZ4a_6Xm60Z7bmbEDADDpwr2e_tch@rabbitmq.default.svc:5672
sending [hello, world!] to queue hello
sending [hello, world!] to queue hello
sending [hello, world!] to queue hello
sending [hello, world!] to queue hello
sending [hello, world!] to queue hello
sending [hello, world!] to queue hello
sending [hello, world!] to queue hello
sending [hello, world!] to queue hello
sending [hello, world!] to queue hello
sending [hello, world!] to queue hello
sending [hello, world!] to queue hello
sending [hello, world!] to queue hello
```

# Resources

If you'd like to learn more about the Service Binding Operator, feel free to
check out the following resources:

- [The Service Binding Operator on GitHub](https://github.com/redhat-developer/service-binding-operator)
- [Official Documentation](https://redhat-developer.github.io/service-binding-operator/)
- [Materials used in this post](https://github.com/sadlerap/sbo-rabbitmq-sample)
