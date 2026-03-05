#!/bin/bash
# Copyright 2025 The Android Open Source Project
#
# script to show status of tap interfaces
# usage: ./tap-status.sh [interface_name]

IFACE=${1:-}

if [ -n "$IFACE" ]; then
    echo "Status for $IFACE:"
    ip addr show "$IFACE"
    ip link show "$IFACE"
else
    echo "All TAP interfaces:"
    ip tuntap list
    echo ""
    echo "Active TAP interfaces details:"
    ip -d link show type tun
fi
