#!/bin/bash
# Copyright 2025 The Android Open Source Project
#
# script to tear down a tap interface for netsim wifi
# usage: ./tap-down.sh <interface_name>

IFACE=${1:-tap0}

echo "Removing TAP interface $IFACE..."
sudo ip link set "$IFACE" down
sudo ip tuntap del dev "$IFACE" mode tap
echo "TAP interface $IFACE removed"
