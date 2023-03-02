# Command Line Interface for Netsim (netsim)

USAGE:
* `netsim [OPTIONS] <SUBCOMMAND>`

OPTIONS:
* `-h, --help`:    Print help information
* `-v, --verbose`: Set verbose mode


## SUBCOMMANDS:
* ### `capture`:    Control the packet capture for one or all devices
    * USAGE:
        * `netsim capture <STATE> <NAME>`

    * ARGS:
        * \<STATE\>:     Capture state [possible values: on, off]
        * \<NAME\>:      Device name
* ### `devices`:    Display device(s) information
    * USAGE:
        * `netsim devices [OPTIONS]`
    * OPTIONS:
        * `-c, --continuous`:    Continuously print device(s) information every second
* ### `help`:       Print this message or the help of the given subcommand(s)
* ### `move`:       Set the device location
    * USAGE:
        * `netsim move <NAME> <X> <Y> [Z]`
    * ARGS:
        * \<NAME\>:      Device name
        * \<X\>:         x position of device
        * \<Y\>:         y position of device
        * \<Z\>:         Optional z position of device
* ### `radio`:      Control the radio state of a device
    * USAGE:
        * `netsim radio <BT_TYPE> <STATUS> <NAME>`

    * ARGS:
        * \<BT_TYPE\>    Radio type [possible values: ble, classic]
        * \<STATUS\>     Radio status [possible values: up, down]
        * \<NAME\>       Device name
* ### `reset`:      Reset Netsim device scene
    * USAGE:
        * `netsim reset`
* ### `version`:    Print Netsim version information
    * USAGE:
        * `netsim version`
* ### `pcap`:       (Not fully implemented)  Control the packet capture functionalities
    * #### SUBCOMMANDS
        * `list`:   List all currently available Pcaps (packet captures)
            * USAGE:
                * `netsim pcap list`
        * `patch`:  Patch a Pcap source to turn packet capture on/off
            * USAGE:
                * `netsim pcap patch <ID> <STATE>`
            * ARGS:
                * \<ID\>        Pcap id
                * \<STATE\>     Packet capture state [possible values: on, off]
        * `get`:    Download the packet capture content
            * USAGE:
                * `netsim pcap get <ID>`
            * ARGS:
                * \<ID\>        Pcap id
