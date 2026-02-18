#!/bin/bash
# Copyright 2025 The Android Open Source Project
#
# script to bring up a tap interface for netsim wifi
# usage: ./tap-up.sh <interface_name> [ip_address/mask]

IFACE=${1:-tap0}
IP=${2:-192.168.10.1/24}

echo "Creating TAP interface $IFACE..."
sudo ip tuntap add dev "$IFACE" mode tap user "$USER"
sudo ip addr add "$IP" dev "$IFACE"
sudo ip link set "$IFACE" up
echo "TAP interface $IFACE created with IP $IP"
