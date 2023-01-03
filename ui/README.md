<p align="center">
  <img width="200" src="https://open-wc.org/hero.png"></img>
</p>

## netsim web UI

[![Built with open-wc recommendations](https://img.shields.io/badge/built%20with-open--wc-blue.svg)](https://github.com/open-wc)

This is a working directory for providing a web UI for netsim.

## Prerequisite

The netsim web server must be up and running. Check go/betosim on how to start multiple virtual devices with netsim.

## Quickstart

To get started:

```sh
npm install
```

```sh
npm start
```

Then open `http://localhost:8000/web/` to run the netsim web UI

## Scripts

- `start` runs your app for development, reloading on file changes

## Tooling configs

For most of the tools, the configuration is in the `package.json` to reduce the amount of files in your project.

If you customize the configuration a lot, you can consider moving them to individual files.

## Authors

[Bill Schilit] schilit@google.com

[Hyun Jae Moon] hyunjaemoon@google.com