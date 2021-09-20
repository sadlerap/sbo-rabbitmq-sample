# RabbitMQ via SBO example

This is an example showing how to connect to rabbitmq using
[SBO](https://github.com/redhat-developer/service-binding-operator).

## Building steps

I tested this with minikube, but it should *effectively* work the same if you're
on something like OpenShift.  This set of steps is to document how to set up
this example on minikube.

1. Start up minikube.
2. Install SBO.  The easiest way to do it is installing it from
   [operatorhub.io](https://operatorhub.io/operator/service-binding-operator)
3. [Install](https://www.rabbitmq.com/kubernetes/operator/quickstart-operator.html)
   the rabbitmq operator onto your cluster.
4. Run the following in a terminal:

``` sh
eval $(minikube docker-env)
docker build -t rabbitmq-test/producer -f Dockerfile.producer .
docker build -t rabbitmq-test/consumer -f Dockerfile.consumer .
```

5. Apply `jobs.yaml`, then `service-binding.yaml`:

``` sh
kubectl apply -f jobs.yaml
kubectl apply -f service-bindings.yaml
```

6. Observe the logs of the consumer pod; you should see messages propogating
   from the producer via a AMQP connection.
