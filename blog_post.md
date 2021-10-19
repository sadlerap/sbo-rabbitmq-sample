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
it needs to expose to the application and will inject it into its runtime
container.  It does so either by environment variables or by files mounted
within the container.

To learn more about some of the other features we support and integrations with
other products, you can read our release announcement
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

First, we need to install Operator Lifecycle Manager (OLM), a prerequisite for
our RabbitMQ operator:
```bash
curl -sL https://github.com/operator-framework/operator-lifecycle-manager/releases/download/v0.19.1/install.sh | bash -s v0.19.1
```

NOTE: yes, doing `curl ... | bash` isn't the best in terms of security.  If this
is a concern for you, you can instead save the installation script to a location
in your filesystem and execute the script from there after inspecting its
contents.

Now we can setup the RabbitMQ operator:
```bash
kubectl apply -f https://github.com/rabbitmq/cluster-operator/releases/latest/download/cluster-operator.yml
```

While we're setting up operators, now would be a good time to install SBO:
```bash
kubectl apply -f https://operatorhub.io/install/service-binding-operator.yaml
```

Next, we want to have our `producer` and `consumer` running on our kubernetes
cluster.  For convenience, I've authored two containers that provide this
functionality; their sources can be found
[here](https://github.com/sadlerap/sbo-rabbitmq-sample).

SBO operates against deployments, so we'll need to make a deployment for each of
our applications.  We can do so with the following:
```yaml
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: producer-deployment
  labels:
    app: producer
spec:
  replicas: 1
  selector:
    matchLabels:
      app: producer
  template:
    metadata:
      labels:
        app: producer
    spec:
      containers:
      - name: producer
        image: quay.io/ansadler/rabbitmq-producer:latest
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: consumer-deployment
  labels:
    app: consumer
spec:
  replicas: 1
  selector:
    matchLabels:
      app: consumer
  template:
    metadata:
      labels:
        app: consumer
    spec:
      containers:
      - name: consumer
        image: quay.io/ansadler/rabbitmq-consumer:latest
```

We could put them in a single deployment, but for the
purposes of this demonstration, I've opted to keep them separate.

We'll also want a rabbitmq cluster to run them against:
```yaml
apiVersion: rabbitmq.com/v1beta1
kind: RabbitmqCluster
metadata:
  name: rabbitmq
  annotations:
    service.binding: path={.status.binding.name},objectType=Secret,elementType=map
spec:
  service:
    type: ClusterIP
```

To make this process easier, you can deploy all of these (that is, `producer`,
`consumer`, and our rabbitmq cluster) with the following:
```bash
kubectl apply -f https://raw.githubusercontent.com/sadlerap/sbo-rabbitmq-sample/master/jobs.yaml
```

NOTE: Look closely at the annotation we've added.  This is how SBO picks up the
information it needs to successfully perform a binding.  If you're interested
in how to read this annotation, check out our
[documentation](https://redhat-developer.github.io/service-binding-operator/userguide/exposing-binding-data/adding-annotation.html).

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
RabbitMQ cluster to either `producer` and `consumer`, so neither of our
containers know where to find it.  Let's fix that.

## Binding things together

If we were not using SBO, we would need to tell both `producer` and `consumer`
how to connect to a rabbitmq instance.  This means distributing at minimum the
following information to these services:

- The hostname of the RabbitMQ instance
- The port that the RabbitMQ instance is listening on
- Authentication credentials (such as username and password)

This would require us to expose our secrets to our applications, either by
having these applications directly fetch that information from kubernetes' API
or by injecting that information into our applications ourselves.  Both of
these methods are rather intrusive into our applications, and it stands to
reason that this process could be automated.

First, we need to allow SBO to make a binding against our RabbitMQ cluster by
adding a ClusterRole.  For security reasons, SBO doesn't have permissions to
make a binding against something it doesn't know about; we can remedy this by
adding a ClusterRole like so:

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: sbo-rabbitmq
  labels:
    servicebinding.io/controller: "true"
rules:
  - apiGroups: ["rabbitmq.com"]
    resources: ["rabbitmqclusters"]
    verbs: ["get", "list"]
```

The `servicebinding.io/controller` label gets the ClusterRole picked up
automatically by SBO; we don't need to add a ClusterRoleBinding as we usually
would need to.

Next, we can perform our bindings.  Service Binding Operator introduces a new
custom resource titled `ServiceBinding`, which represents the binding between
an application and a service.  In this particular example, the bindings for our
`producer` and `consumer` applications would look like this:
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

# An even easier way

We don't always need to specify these annotations on our custom resources.
Instead, we could instead set the label `servicebinding.io/provisioned-service:
true` on the custom resource (instead of the annotations we would usually set)
and everything should work.  Ideally, this would already be done for us;
however, at time of writing, this label has not been applied to RabbitMQ's
operator.

# Resources

- [The Service Binding Operator on GitHub](https://github.com/redhat-developer/service-binding-operator)
- [Official Documentation](https://redhat-developer.github.io/service-binding-operator/)
- [Materials used in this post](https://github.com/sadlerap/sbo-rabbitmq-sample)
