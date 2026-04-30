---
title: About Edgerun
summary: Why Edgerun exists and how it is being built.
---
# About Edgerun

Edgerun starts from a simple concern: building software on top of layer after layer of opaque abstraction is becoming infeasible. The machine underneath keeps getting faster, but the systems we ask people to operate keep getting heavier, more fragile, and harder to understand.

The old line often attributed to Bill Gates, "640K ought to be enough for anybody," is probably apocryphal. Still, it is useful as a warning. It captures how quickly software expands to consume whatever hardware budget is available. My own experience configuring a Kubernetes platform before deploying a single actual application was already using around 12 GB of RAM. The source checkout for Chrome is roughly tens of gigabytes. Android is hundreds of gigabytes, and building it can add hundreds more. These numbers are normal now, but they should not feel normal.

We have an enormous amount of compute available. For normal human needs, the world already has more than enough CPUs, memory, storage, and network capacity. The problem is that modern software practice wastes a shocking amount of it before useful work even begins. Every extra layer asks for its own control plane, runtime, image format, cache, log stream, package graph, dashboard, and failure mode.

Edgerun is an experiment in taking responsibility for more of the stack again: identity, networking, storage, email, sites, code publishing, and the operational tools around them. Not because every abstraction is bad, but because abstractions should be small enough to inspect, cheap enough to run, and honest about the costs they impose.

The goal is to find out how far we can reduce our dependence on huge, polluting datacenters by moving toward efficient peer-to-peer systems and software that can run close to the people using it. That means fewer mandatory platforms, less idle machinery, less hidden complexity, and more systems that can be understood by one person or a small group.

This started as a practical exploration, not a finished manifesto. In this video I was still experimenting with Kubernetes for the idea, before pushing harder on the question of how much of that machinery should exist at all:

https://www.youtube.com/watch?v=AIMdIAoiR80

I am easier to reach by email than anywhere else. The only reliable way to contact me is [ken@edgerun.tech](mailto:ken@edgerun.tech).

The blog will document this feature by feature as pieces become public. The code explorer will expose the matching code in controlled slices, so posts can point at real commits and real files instead of vague descriptions.
