## Verify: A Software Validation Framework

> "Vibe, but verify." &mdash; Ronald Reagan

Verify is a way to bridge **Human Intentions** and **Generative AI Sofware**
using a set of tools and libraries.

### 1. The Gherkin Language

This is a domain specific language widely used by the BDD community that allows
you to define the features of your software in plain English.

The current implementation uses Gherkin syntax with these additions:

1. Actors, where the steps are run, can be specified in the feature file.
2. Parameters can be expanded in steps using interpolation.

### 1. Actors

A specific environment or state where the Intention step is executed. Actors are
specified in the feature file:

```
Feature: mDNS Service Discovery

  Scenario: Basic mDNS Service Discovery
    GIVEN @actor_name is advertising an mDNS service: "foo"
    WHEN @actor_name discovers mDNS services
    THEN @actor_name should discover the mDNS {service}
```

#### Actor Ecosystem

The runner will register an initial set of actors:

| Actor     | Description              |
| :-------- | :----------------------- |
| `adb`     | The Android Debug Bridge |
| `network` | The netsim network       |
| `host`    | The host machine         |

[NYI] The `adb` actor will register the following actors:

| Actor    | Description                             |
| :------- | :-------------------------------------- |
| `avd`    | The first AVD running                   |
| `phone`  | The first AVD running property `phone`  |
| `tablet` | The first AVD running property `tablet` |
| `watch`  | The first AVD running property `watch`  |
| `tv`     | The first AVD running property `tv`     |

If more than one actor of the type is registered, the actor will be named
`avd:1`, `avd:2`, etc.

[NYI] The `network` actor will register the following actors:

| Actor    | Description                                  |
| :------- | :------------------------------------------- |
| `ap`     | The first Access Point running property `ap` |
| `beacon` | The first Beacon running property `beacon`   |

If more than one actor of the type is registered, the actor will be named
`ap:SSID`, `ap:SSID2`, etc.

- _Example:_ "User uploads a 1GB file via the API."

### 2. Sharing Parameters across actors

Interpolation is used to share parameters across actors. For example, if an
actor creates a TCP port, it can be shared with another actor using
interpolation.

```
GIVEN @actor_name creates echo service on tcp_port
THEN @actor_name should echo {tcp_port}
```

TODO: currently the actor decides the names of the parameter keys which may be a
problem in multi-actor scenarios.

### 3. Codegen

Each line in the feature file is translated into a Rust function call.

The codegen reads the feature files and generates the Rust code that will be
used to run the tests.

Comments are used to provide additional information to the codegen.

Example:

### 4. Agents

The piece of code that lives inside the system being tested.

- It **Executes** the steps of the Scene.
- It **Validates** that the Rails weren't crossed.
- In a backend, the Agent might be a sidecar container. In a library, it might
  be a wrapper.

### 5. The Runner

The piece of code that lives inside the system being tested.

- It **Executes** the steps of the Scene.
- It **Validates** that the Rails weren't crossed.
- In a backend, the Agent might be a sidecar container. In a library, it might
  be a wrapper.

---

## Why this works for _any_ Software

| Testing Level   | How Intentions handles it                                                                                 |
| --------------- | --------------------------------------------------------------------------------------------------------- |
| **Integration** | The **Agent** probes internal APIs and state to prove the logic is sound.                                 |
| **End-to-End**  | The **Agent** interacts with the external interface (API, CLI, or UI) to prove the experience is correct. |
| **Distributed** | Multiple **Agents** coordinate across different servers/services to prove the system-wide intent.         |
