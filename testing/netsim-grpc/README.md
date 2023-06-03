# Netsim python gRPC library

This contains a python library to interact with Netsim gRPC frontend service

## Development

If you wish to do development you can create a virtual environment and
generate python files from .proto files by running:

     . ./configure.sh

## Adding dependencies

Configure will use the local python interpreter, which does not
have TLS support, so all the package must be made available locally!

If you need to add a package, make the source package available under the
repo directory. The easiest way to do this is:

     pip3 install pip2pi
     pip3 download  --no-binary ":all:"  my-package=1.2.3 -d repo
     dir2pi repo

## Usage

TODO