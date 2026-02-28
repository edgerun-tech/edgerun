# Eventbus Crate Executors

## Trigger and status subjects

- Trigger: `edgerun.code.updated`
- Build status: `edgerun.executors.<crate>.build.status`
- Test status: `edgerun.executors.<crate>.test.status`

## Local run

```bash
scripts/executors/run-crate-executor.sh edgerun-event-bus build 0
scripts/executors/run-crate-executor.sh edgerun-event-bus test 0
```

## Publish a code update event

```bash
scripts/executors/publish-code-update.sh
```

## Generate/deploy swarm stack

```bash
scripts/swarm/generate-crate-executors-stack.sh
scripts/swarm/deploy-crate-executors-stack.sh
```

## Add worker node

```bash
scripts/swarm/add-worker-node.sh 10.13.37.2
```
