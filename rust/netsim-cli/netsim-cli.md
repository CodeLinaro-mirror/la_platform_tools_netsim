# Command Line Interface for Netsim (netsim)

USAGE:
* `netsim <SUBCOMMAND>`

OPTIONS:
* `-h, --help`:    Print help information

## SUBCOMMANDS:
* ### `capture`:           Control the packet capture for one or all devices
    * USAGE:
        * `netsim capture <STATE> <DEVICE_SERIAL>`

    * ARGS:
        * \<STATE\>:     Capture state [possible values: on, off]
        * \<DEVICE_SERIAL\>:    Device serial
* ### `devices`:           Display device(s) information
    * USAGE:
        * `netsim devices [OPTIONS]`
    * OPTIONS:
        * `-c, --continuous`:    Continuously print device(s) information every second
* ### `help`:              Print this message or the help of the given subcommand(s)
* ### `move`:              Set the device location
    * USAGE:
        * `netsim move <DEVICE_SERIAL> <X> <Y> [Z]`
    * ARGS:
        * \<DEVICE_SERIAL\>:    Device serial
        * \<X\>:         x position of device
        * \<Y\>:         y position of device
        * \<Z\>:         Optional z position of device
* ### `radio`:             Control the radio state of a device
    * USAGE:
        * `netsim radio <BT_TYPE> <STATUS> <DEVICE_SERIAL>`

    * ARGS:
        * \<BT_TYPE\>    Radio type [possible values: ble, classic]
        * \<STATUS\>     Radio status [possible values: up, down]
        * \<DEVICE_SERIAL\>     Device serial
* ### `reset`:             Reset Netsim device scene
    * USAGE:
        * `netsim reset`
* ### `version`:          Print Netsim version information
    * USAGE:
        * `netsim version`
