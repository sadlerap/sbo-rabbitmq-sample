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

In summary, we want our setup to look like the following:

- An operator-managed RabbitMQ cluster running on kubernetes (we will use
  https://github.com/rabbitmq/cluster-operator for this demonstration)
- Our `producer` and `consumer` also running on kubernetes

# Connecting made easier

Instead of connecting the hard way, we can leverage SBO to get these services
to talk to RabbitMQ.

First, let's get our rabbitmq cluster setup.  First, we need to install
Operator Lifecycle Manager (OLM), a prerequisite for our RabbitMQ operator:
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

## Binding things together

Right now, we have our applications running, but they're currently not talking
to our RabbitMQ cluster yet.  Let's fix that by making a binding request.

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

Next, we can perform our bindings:
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
been receiving messages from our `producer`.

# An even easier way

We don't always need to specify these annotations on our custom resources.
Instead, we could instead set the label `servicebinding.io/provisioned-service:
true` on the custom resource (instead of the annotations we would usually set)
and everything should work.  Ideally, this would already be done for us;
however, at time of writing, this label has not already been set on RabbitMQ's
operator.

# Resources

- [The Service Binding Operator on GitHub](https://github.com/redhat-developer/service-binding-operator)
- [Official Documentation](https://redhat-developer.github.io/service-binding-operator/)
